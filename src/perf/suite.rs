//! Git operation timing suite (issue #34).

use std::path::Path;
use std::time::Instant;

use serde::Serialize;

use crate::git::service::GitService;
use crate::git::{blame, diff, history, GixService};
use crate::perf::measure_ms;
use crate::perf::thresholds::{Budget, Thresholds};
use crate::perf::workers::worker_count;
use crate::task::{Priority, TaskRunner};

/// One timed gitbolt operation.
#[derive(Debug, Clone, Serialize)]
pub struct Sample {
    pub name: String,
    pub ms: f64,
    pub budget: String,
}

/// Full suite report (JSON-serializable).
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub repo: String,
    pub workers: usize,
    pub samples: Vec<Sample>,
    pub rss_kib: Option<u64>,
    pub rss_budget: Option<String>,
    pub worker_sweep: Option<Vec<WorkerSweepPoint>>,
    pub recommended_workers: Option<usize>,
    pub warnings: Vec<String>,
    pub failures: Vec<String>,
}

/// Throughput sample for a worker-count candidate.
#[derive(Debug, Clone, Serialize)]
pub struct WorkerSweepPoint {
    pub workers: usize,
    pub wall_ms: f64,
    pub jobs: usize,
}

fn budget_label(b: Budget) -> String {
    match b {
        Budget::Ok => "ok".into(),
        Budget::Warn => "warn".into(),
        Budget::Fail => "fail".into(),
    }
}

struct Collect {
    samples: Vec<Sample>,
    warnings: Vec<String>,
    failures: Vec<String>,
}

impl Collect {
    fn push(&mut self, name: &str, ms: f64, warn: f64, fail: f64) {
        let budget = Budget::classify(ms, warn, fail);
        let label = budget_label(budget);
        if budget == Budget::Warn {
            self.warnings
                .push(format!("{name} {ms:.2}ms exceeds warn {warn}ms"));
        }
        if budget == Budget::Fail {
            self.failures
                .push(format!("{name} {ms:.2}ms exceeds fail {fail}ms"));
        }
        self.samples.push(Sample {
            name: name.into(),
            ms,
            budget: label,
        });
    }
}

/// Runs open / status / diff / blame / history against `repo`.
#[must_use]
#[allow(clippy::missing_panics_doc, clippy::too_many_lines)]
pub fn run_suite(repo: &Path, thresholds: &Thresholds, sweep: bool) -> Report {
    let mut c = Collect {
        samples: Vec::new(),
        warnings: Vec::new(),
        failures: Vec::new(),
    };

    let (svc, open_ms) = measure_ms(|| GixService::open(repo).expect("open repo"));
    c.push(
        "open",
        open_ms,
        thresholds.open_ms_warn,
        thresholds.open_ms_fail,
    );

    let ((), status_ms) = measure_ms(|| {
        let _ = svc.status().expect("status");
    });
    c.push(
        "status",
        status_ms,
        thresholds.status_ms_warn,
        thresholds.status_ms_fail,
    );

    let probe = Path::new("src/f0000.txt");
    let ((), diff_ms) = measure_ms(|| {
        let _ = diff::unified_diff(repo, probe, false);
    });
    c.push(
        "diff",
        diff_ms,
        thresholds.diff_ms_warn,
        thresholds.diff_ms_fail,
    );

    let ((), blame_ms) = measure_ms(|| {
        let _ = blame::blame_lines(repo, probe, &[1, 2, 3]);
    });
    c.push(
        "blame",
        blame_ms,
        thresholds.blame_ms_warn,
        thresholds.blame_ms_fail,
    );

    let ((), history_ms) = measure_ms(|| {
        let _ = history::log_page(repo, 0, 100);
    });
    c.push(
        "history",
        history_ms,
        thresholds.history_ms_warn,
        thresholds.history_ms_fail,
    );

    let rss = crate::perf::rss_kib();
    let rss_budget = rss.map(|v| {
        let b = Budget::classify_u64(v, thresholds.rss_kib_warn, thresholds.rss_kib_fail);
        if b == Budget::Warn {
            c.warnings.push(format!(
                "rss {v} KiB exceeds warn {}",
                thresholds.rss_kib_warn
            ));
        }
        if b == Budget::Fail {
            c.failures.push(format!(
                "rss {v} KiB exceeds fail {}",
                thresholds.rss_kib_fail
            ));
        }
        budget_label(b)
    });

    let (worker_sweep, recommended_workers) = if sweep {
        let points = sweep_workers(repo, &[1, 2, 4, 6, 8]);
        let recommended = recommend_workers(&points);
        (Some(points), recommended)
    } else {
        (None, None)
    };

    Report {
        repo: repo.display().to_string(),
        workers: worker_count(),
        samples: c.samples,
        rss_kib: rss,
        rss_budget,
        worker_sweep,
        recommended_workers,
        warnings: c.warnings,
        failures: c.failures,
    }
}

/// Times draining `jobs` status calls across `worker` counts.
fn sweep_workers(repo: &Path, counts: &[usize]) -> Vec<WorkerSweepPoint> {
    let jobs = 24usize;
    let mut out = Vec::new();
    for &n in counts {
        let (runner, rx) = TaskRunner::new(n);
        let start = Instant::now();
        for _ in 0..jobs {
            let path = repo.to_path_buf();
            runner.submit(Priority::P1, crate::app::model::Generation(1), move || {
                let _ = GixService::open(&path).and_then(|s| s.status());
                0u8
            });
        }
        let mut done = 0;
        while done < jobs {
            let _ = rx.recv();
            done += 1;
        }
        drop(runner);
        out.push(WorkerSweepPoint {
            workers: n,
            wall_ms: crate::perf::duration_ms(start.elapsed()),
            jobs,
        });
    }
    out
}

/// Picks the smallest worker count within 10% of the best wall time.
fn recommend_workers(points: &[WorkerSweepPoint]) -> Option<usize> {
    let best = points
        .iter()
        .map(|p| p.wall_ms)
        .fold(f64::INFINITY, f64::min);
    if !best.is_finite() {
        return None;
    }
    points
        .iter()
        .filter(|p| p.wall_ms <= best * 1.10)
        .map(|p| p.workers)
        .min()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perf::fixture::BenchRepo;

    #[test]
    fn suite_runs_on_scale_fixture() {
        let repo = BenchRepo::scale(8, 3).expect("fixture");
        let report = run_suite(repo.path(), &Thresholds::default(), false);
        assert_eq!(report.samples.len(), 5);
        assert!(report.failures.is_empty(), "{:?}", report.failures);
    }

    #[test]
    fn recommend_prefers_smallest_near_best() {
        let points = vec![
            WorkerSweepPoint {
                workers: 1,
                wall_ms: 100.0,
                jobs: 10,
            },
            WorkerSweepPoint {
                workers: 2,
                wall_ms: 55.0,
                jobs: 10,
            },
            WorkerSweepPoint {
                workers: 4,
                wall_ms: 50.0,
                jobs: 10,
            },
            WorkerSweepPoint {
                workers: 8,
                wall_ms: 49.0,
                jobs: 10,
            },
        ];
        assert_eq!(recommend_workers(&points), Some(4));
    }
}
