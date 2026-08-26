//! Performance PoC for GitBolt (issue #2).
//!
//! A deliberately minimal Dioxus Desktop app that opens a repository and shows
//! its `git status` as a list. It exists to measure whether the performance
//! targets in `docs/design/02-tech-and-performance.md` (startup, memory, UI
//! latency) are reachable with Dioxus Desktop.
//!
//! Modes:
//! - default: render the status list interactively.
//! - `POC_BENCH=1`: run an automated benchmark (time-to-first-render, resident
//!   memory, and a signal-update -> re-render latency proxy), print the results
//!   as JSON to stdout, and exit.
//!
//! Configuration via environment variables:
//! - `POC_REPO`: repository path to inspect (defaults to the current directory).
//! - `POC_BENCH`: when `1`, run the benchmark and exit.
//! - `POC_ITERS`: number of latency iterations in benchmark mode (default 200).

use std::process::Command;
use std::sync::OnceLock;
use std::time::Instant;

use dioxus::prelude::*;

/// Process start instant, captured as early as possible in `main`.
static START: OnceLock<Instant> = OnceLock::new();
/// Data gathered before launching the UI.
static POC: OnceLock<PocData> = OnceLock::new();

/// A single working-tree status entry (porcelain v1 XY + path).
#[derive(Clone)]
struct StatusEntry {
    code: String,
    path: String,
}

/// Everything the PoC UI needs, computed once before launch.
struct PocData {
    repo: String,
    branch: String,
    entries: Vec<StatusEntry>,
}

fn main() {
    let _ = START.set(Instant::now());

    let repo = std::env::var("POC_REPO")
        .ok()
        .or_else(|| std::env::args().nth(1))
        .unwrap_or_else(|| ".".to_string());

    let data = PocData {
        branch: read_branch(&repo),
        entries: read_status(&repo),
        repo,
    };
    let _ = POC.set(data);

    dioxus::launch(app);
}

/// Reads the current branch name via `git`, tolerating detached HEAD / errors.
fn read_branch(repo: &str) -> String {
    run_git(repo, &["rev-parse", "--abbrev-ref", "HEAD"])
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "-".to_string())
}

/// Reads `git status --porcelain=v1 -z` and parses it into entries.
fn read_status(repo: &str) -> Vec<StatusEntry> {
    let Some(out) = run_git(repo, &["status", "--porcelain=v1", "-z"]) else {
        return Vec::new();
    };

    let mut entries = Vec::new();
    let mut parts = out.split('\0');
    while let Some(record) = parts.next() {
        if record.len() < 3 {
            continue;
        }
        let code = record[..2].to_string();
        let path = record[3..].to_string();
        // Renames/copies encode the source path as a following NUL field.
        if code.starts_with('R') || code.starts_with('C') {
            let _ = parts.next();
        }
        entries.push(StatusEntry { code, path });
    }
    entries
}

/// Runs a git subcommand, returning stdout as a `String` on success.
fn run_git(repo: &str, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        None
    }
}

/// Reads a `VmRSS`/`VmHWM`-style value (in kB) from `/proc/self/status`.
#[cfg(target_os = "linux")]
fn read_proc_kib(field: &str) -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix(field) {
            return rest
                .trim()
                .trim_end_matches("kB")
                .trim()
                .parse::<u64>()
                .ok();
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
fn read_proc_kib(_field: &str) -> Option<u64> {
    None
}

/// Root component: repository pulse header + status list, plus the optional
/// benchmark driver.
fn app() -> Element {
    let data = POC.get().expect("PoC data initialized before launch");

    let bench = std::env::var("POC_BENCH").as_deref() == Ok("1");
    if bench {
        drive_benchmark(data.entries.len());
    }

    let staged = data.entries.iter().filter(|e| is_staged(&e.code)).count();

    rsx! {
        div {
            font_family: "-apple-system, system-ui, sans-serif",
            font_size: "13px",
            padding: "8px",
            div {
                font_weight: "600",
                padding_bottom: "6px",
                "GitBolt PoC / {data.branch} · {data.entries.len()} changes · {staged} staged"
            }
            div {
                font_size: "11px",
                color: "#888",
                padding_bottom: "8px",
                "{data.repo}"
            }
            for entry in data.entries.iter().take(500) {
                div {
                    key: "{entry.path}",
                    display: "flex",
                    gap: "8px",
                    font_family: "monospace",
                    span { width: "24px", color: "#3b82f6", "{entry.code}" }
                    span { "{entry.path}" }
                }
            }
        }
    }
}

/// Returns true when the porcelain XY code indicates a staged change.
fn is_staged(code: &str) -> bool {
    matches!(code.as_bytes().first(), Some(c) if !matches!(c, b' ' | b'?'))
}

/// Installs an effect that measures signal-update -> re-render latency, then
/// reports startup / memory / latency stats and exits the process.
fn drive_benchmark(entry_count: usize) {
    let iters: usize = std::env::var("POC_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200);

    let mut tick = use_signal(|| 0usize);
    let mut samples = use_signal(Vec::<u128>::new);
    // Non-reactive holder for the start instant of the pending cycle.
    let mut last = use_signal(|| None::<Instant>);

    use_effect(move || {
        let i = tick(); // subscribe to `tick` only

        if let Some(start) = *last.peek() {
            samples.write().push(start.elapsed().as_micros());
        }

        if i >= iters {
            report(entry_count, iters, &samples.read());
            std::process::exit(0);
        }

        *last.write() = Some(Instant::now());
        // Defer the next increment so the current render commits first.
        spawn(async move {
            *tick.write() += 1;
        });
    });
}

/// Prints benchmark results as a single JSON line to stdout.
fn report(entry_count: usize, iters: usize, samples: &[u128]) {
    let first_render_ms = START.get().map_or(0, |s| s.elapsed().as_millis());

    let (min, med, max, avg) = summarize(samples);
    let rss = read_proc_kib("VmRSS:").unwrap_or(0);
    let peak = read_proc_kib("VmHWM:").unwrap_or(0);

    println!(
        "{{\"time_to_first_render_ms\":{first_render_ms},\
\"rss_kib\":{rss},\"peak_rss_kib\":{peak},\
\"status_entries\":{entry_count},\"latency_iters\":{iters},\
\"update_latency_us\":{{\"min\":{min},\"median\":{med},\"max\":{max},\"avg\":{avg}}}}}"
    );
}

/// Computes (min, median, max, avg) of a slice of microsecond samples.
fn summarize(samples: &[u128]) -> (u128, u128, u128, u128) {
    if samples.is_empty() {
        return (0, 0, 0, 0);
    }
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let min = sorted[0];
    let max = sorted[sorted.len() - 1];
    let med = sorted[sorted.len() / 2];
    let sum: u128 = sorted.iter().sum();
    let avg = sum / sorted.len() as u128;
    (min, med, max, avg)
}
