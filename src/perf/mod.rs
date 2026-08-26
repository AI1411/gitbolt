//! Performance measurement helpers and thresholds (issue #34).
//!
//! See `docs/design/02-tech-and-performance.md` §3 and `07-runtime.md` (worker
//! count tuned via benchmark).

pub mod fixture;
pub mod suite;
pub mod thresholds;
pub mod workers;

use std::time::{Duration, Instant};

/// Wall-clock timing of `f` in milliseconds (fractional).
pub fn measure_ms<R>(f: impl FnOnce() -> R) -> (R, f64) {
    let start = Instant::now();
    let out = f();
    (out, duration_ms(start.elapsed()))
}

/// Converts a duration to milliseconds as `f64`.
#[must_use]
pub fn duration_ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1_000.0
}

/// Best-effort resident set size in KiB (Linux `/proc`, else `ps`).
#[must_use]
pub fn rss_kib() -> Option<u64> {
    if let Some(v) = read_proc_kib("VmRSS:") {
        return Some(v);
    }
    read_ps_rss_kib()
}

fn read_proc_kib(key: &str) -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix(key) {
            let num = rest.split_whitespace().next()?;
            return num.parse().ok();
        }
    }
    None
}

fn read_ps_rss_kib() -> Option<u64> {
    let pid = std::process::id().to_string();
    let output = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &pid])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measure_ms_returns_non_negative() {
        let (v, ms) = measure_ms(|| 42);
        assert_eq!(v, 42);
        assert!(ms >= 0.0);
    }
}
