//! Priority-aware background task runtime.
//!
//! Implements the P0–P3 scheduling model from `docs/design/07-runtime.md`
//! section 12. Git and IO work runs on a bounded pool of worker threads so the
//! UI thread is never blocked and the process never spawns an unbounded number
//! of threads. Results are delivered to the UI thread over a channel as
//! [`Outcome`]s tagged with the [`Generation`] they were submitted under, so
//! stale results can be discarded (both here, by skipping superseded tasks, and
//! downstream in the reducer).

use std::collections::BinaryHeap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::JoinHandle;

use crate::app::model::Generation;

/// Task priority. `P0` is the most urgent; `P3` the least.
///
/// | Priority | Target |
/// |----------|--------|
/// | P0 | User actions (commit, stage) |
/// | P1 | Visible UI (selected file diff) |
/// | P2 | Near-visible (next history page) |
/// | P3 | Hidden metadata (off-screen branch info) |
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    P0,
    P1,
    P2,
    P3,
}

impl Priority {
    /// Numeric rank where a smaller value is more urgent.
    #[must_use]
    pub fn rank(self) -> u8 {
        match self {
            Self::P0 => 0,
            Self::P1 => 1,
            Self::P2 => 2,
            Self::P3 => 3,
        }
    }
}

/// A completed task's result, tagged with its submission generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome<M> {
    pub generation: Generation,
    pub message: M,
}

/// A snapshot of runtime activity for reflecting into `BackgroundTaskState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TaskStats {
    /// Tasks waiting in the queue.
    pub queued: usize,
    /// Tasks currently executing on a worker.
    pub running: usize,
}

type Job<M> = Box<dyn FnOnce() -> M + Send>;

struct QueuedTask<M> {
    priority: Priority,
    seq: u64,
    generation: Generation,
    job: Job<M>,
}

// Ordering considers only (priority, seq); the job is not comparable.
// `BinaryHeap` is a max-heap, so "greater" means "scheduled sooner": a more
// urgent priority (smaller rank) or, on ties, an earlier sequence number.
impl<M> PartialEq for QueuedTask<M> {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.seq == other.seq
    }
}
impl<M> Eq for QueuedTask<M> {}
impl<M> PartialOrd for QueuedTask<M> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<M> Ord for QueuedTask<M> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .priority
            .rank()
            .cmp(&self.priority.rank())
            .then_with(|| other.seq.cmp(&self.seq))
    }
}

struct Inner<M> {
    heap: BinaryHeap<QueuedTask<M>>,
    seq: u64,
    generation: Generation,
    running: usize,
    shutdown: bool,
}

struct Shared<M> {
    inner: Mutex<Inner<M>>,
    signal: Condvar,
}

/// A bounded pool of worker threads that execute submitted tasks by priority.
pub struct TaskRunner<M: Send + 'static> {
    shared: Arc<Shared<M>>,
    workers: Vec<JoinHandle<()>>,
}

impl<M: Send + 'static> TaskRunner<M> {
    /// Creates a runner with `workers` threads (at least one) and returns it
    /// alongside the receiver of task [`Outcome`]s.
    ///
    /// # Panics
    /// Panics if a worker thread cannot be spawned.
    #[must_use]
    pub fn new(workers: usize) -> (Self, Receiver<Outcome<M>>) {
        let workers = workers.max(1);
        let (tx, rx) = mpsc::channel();
        let shared = Arc::new(Shared {
            inner: Mutex::new(Inner {
                heap: BinaryHeap::new(),
                seq: 0,
                generation: Generation::default(),
                running: 0,
                shutdown: false,
            }),
            signal: Condvar::new(),
        });

        let handles = (0..workers)
            .map(|i| {
                let shared = Arc::clone(&shared);
                let tx = tx.clone();
                std::thread::Builder::new()
                    .name(format!("gitbolt-worker-{i}"))
                    .spawn(move || worker_loop(&shared, &tx))
                    .expect("spawn worker thread")
            })
            .collect();

        (
            Self {
                shared,
                workers: handles,
            },
            rx,
        )
    }

    /// Submits a task at the given priority and generation.
    pub fn submit<F>(&self, priority: Priority, generation: Generation, job: F)
    where
        F: FnOnce() -> M + Send + 'static,
    {
        let mut inner = lock(&self.shared.inner);
        inner.seq += 1;
        let seq = inner.seq;
        inner.heap.push(QueuedTask {
            priority,
            seq,
            generation,
            job: Box::new(job),
        });
        drop(inner);
        self.shared.signal.notify_one();
    }

    /// Advances the current generation. Queued tasks submitted under an older
    /// generation are dropped (not executed) when a worker reaches them.
    pub fn set_generation(&self, generation: Generation) {
        let mut inner = lock(&self.shared.inner);
        if generation > inner.generation {
            inner.generation = generation;
        }
        drop(inner);
        self.shared.signal.notify_all();
    }

    /// Returns a snapshot of the queued and running task counts.
    #[must_use]
    pub fn stats(&self) -> TaskStats {
        let inner = lock(&self.shared.inner);
        TaskStats {
            queued: inner.heap.len(),
            running: inner.running,
        }
    }
}

impl<M: Send + 'static> Drop for TaskRunner<M> {
    fn drop(&mut self) {
        {
            let mut inner = lock(&self.shared.inner);
            inner.shutdown = true;
        }
        self.shared.signal.notify_all();
        for handle in self.workers.drain(..) {
            let _ = handle.join();
        }
    }
}

/// Locks a mutex, recovering the guard if a previous holder panicked.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn worker_loop<M: Send + 'static>(shared: &Arc<Shared<M>>, tx: &Sender<Outcome<M>>) {
    loop {
        let task = {
            let mut inner = lock(&shared.inner);
            loop {
                if let Some(top) = inner.heap.pop() {
                    // Drop tasks superseded by a newer generation.
                    if top.generation < inner.generation {
                        continue;
                    }
                    inner.running += 1;
                    break top;
                }
                if inner.shutdown {
                    return;
                }
                inner = shared
                    .signal
                    .wait(inner)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
        };

        let outcome = Outcome {
            generation: task.generation,
            message: (task.job)(),
        };
        let _ = tx.send(outcome);

        let mut inner = lock(&shared.inner);
        inner.running -= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;
    use std::time::Duration;

    fn wait_until<F: Fn() -> bool>(predicate: F) {
        for _ in 0..1000 {
            if predicate() {
                return;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        panic!("condition not met within timeout");
    }

    #[test]
    fn tasks_run_in_priority_then_fifo_order() {
        let (runner, rx) = TaskRunner::<&'static str>::new(1);
        let (gate_tx, gate_rx) = channel::<()>();

        // Occupy the single worker with a gate task so the rest queue up.
        runner.submit(Priority::P0, Generation(0), move || {
            gate_rx.recv().expect("gate release");
            "gate"
        });
        wait_until(|| runner.stats().running == 1);

        // Submit out of priority order (and two at the same priority).
        runner.submit(Priority::P3, Generation(0), || "p3");
        runner.submit(Priority::P1, Generation(0), || "p1a");
        runner.submit(Priority::P1, Generation(0), || "p1b");
        runner.submit(Priority::P0, Generation(0), || "p0");
        wait_until(|| runner.stats().queued == 4);

        gate_tx.send(()).expect("release gate");

        let mut order = Vec::new();
        for _ in 0..5 {
            order.push(rx.recv().expect("outcome").message);
        }
        // gate runs first (already executing), then strict priority, FIFO on ties.
        assert_eq!(order, vec!["gate", "p0", "p1a", "p1b", "p3"]);
    }

    #[test]
    fn stale_generation_tasks_are_dropped() {
        let (runner, rx) = TaskRunner::<u32>::new(1);
        let (gate_tx, gate_rx) = channel::<()>();

        runner.submit(Priority::P0, Generation(1), move || {
            gate_rx.recv().expect("gate release");
            0
        });
        wait_until(|| runner.stats().running == 1);

        // Queue a stale task, then advance the generation and queue a fresh one.
        runner.submit(Priority::P1, Generation(1), || 111);
        runner.set_generation(Generation(2));
        runner.submit(Priority::P1, Generation(2), || 222);
        wait_until(|| runner.stats().queued == 2);

        gate_tx.send(()).expect("release gate");

        // gate (0) then the fresh task (222); the stale 111 is skipped.
        assert_eq!(rx.recv().expect("gate outcome").message, 0);
        let fresh = rx.recv().expect("fresh outcome");
        assert_eq!(fresh.message, 222);
        assert_eq!(fresh.generation, Generation(2));
        assert!(rx.recv_timeout(Duration::from_millis(100)).is_err());
    }

    #[test]
    fn multiple_workers_process_all_tasks() {
        let (runner, rx) = TaskRunner::<u32>::new(4);
        for i in 0..100u32 {
            runner.submit(Priority::P2, Generation(0), move || i);
        }
        let mut sum = 0u32;
        for _ in 0..100 {
            sum += rx.recv().expect("outcome").message;
        }
        assert_eq!(sum, (0..100u32).sum::<u32>());
    }
}
