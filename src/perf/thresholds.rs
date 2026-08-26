//! Soft / hard thresholds for CI regression warnings (issue #34).

/// Named timing budgets for gitbolt-bench.
#[derive(Debug, Clone, Copy)]
pub struct Thresholds {
    pub open_ms_warn: f64,
    pub open_ms_fail: f64,
    pub status_ms_warn: f64,
    pub status_ms_fail: f64,
    pub diff_ms_warn: f64,
    pub diff_ms_fail: f64,
    pub blame_ms_warn: f64,
    pub blame_ms_fail: f64,
    pub history_ms_warn: f64,
    pub history_ms_fail: f64,
    pub rss_kib_warn: u64,
    pub rss_kib_fail: u64,
}

impl Default for Thresholds {
    fn default() -> Self {
        // Synthetic fixture budgets — generous vs design targets so CI noise
        // stays at warning level; fail only on pathological slowdowns.
        Self {
            open_ms_warn: 250.0,
            open_ms_fail: 5_000.0,
            status_ms_warn: 200.0,
            status_ms_fail: 3_000.0,
            diff_ms_warn: 250.0,
            diff_ms_fail: 3_000.0,
            blame_ms_warn: 400.0,
            blame_ms_fail: 5_000.0,
            history_ms_warn: 300.0,
            history_ms_fail: 3_000.0,
            rss_kib_warn: 200_000, // ~200 MiB
            rss_kib_fail: 512_000,
        }
    }
}

/// Outcome of comparing a sample to warn/fail budgets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Budget {
    Ok,
    Warn,
    Fail,
}

impl Budget {
    /// Classifies `value` against warn/fail ceilings (fail wins).
    #[must_use]
    pub fn classify(value: f64, warn: f64, fail: f64) -> Self {
        if value > fail {
            Self::Fail
        } else if value > warn {
            Self::Warn
        } else {
            Self::Ok
        }
    }

    /// Classifies an integer sample (RSS).
    #[must_use]
    pub fn classify_u64(value: u64, warn: u64, fail: u64) -> Self {
        if value > fail {
            Self::Fail
        } else if value > warn {
            Self::Warn
        } else {
            Self::Ok
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_orders_ok_warn_fail() {
        assert_eq!(Budget::classify(1.0, 10.0, 100.0), Budget::Ok);
        assert_eq!(Budget::classify(50.0, 10.0, 100.0), Budget::Warn);
        assert_eq!(Budget::classify(150.0, 10.0, 100.0), Budget::Fail);
    }
}
