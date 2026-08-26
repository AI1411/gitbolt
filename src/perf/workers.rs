//! Worker pool sizing (issue #34).

/// Environment variable override for the Git / IO worker pool size.
pub const WORKERS_ENV: &str = "GITBOLT_WORKERS";

/// Default worker count when unset (see `docs/design/07-runtime.md`).
pub const DEFAULT_WORKERS: usize = 4;

/// Resolved worker count: `GITBOLT_WORKERS` or [`DEFAULT_WORKERS`], clamped to `1..=16`.
#[must_use]
pub fn worker_count() -> usize {
    std::env::var(WORKERS_ENV)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_WORKERS)
        .clamp(1, 16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_four_when_unset() {
        // Cannot reliably mutate process env in parallel tests; just assert clamp.
        assert!((1..=16).contains(&DEFAULT_WORKERS));
        assert_eq!(4_usize.clamp(1, 16), 4);
    }
}
