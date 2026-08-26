//! Blame formatting helpers for Smart Blame (issue #22).

use crate::app::model::CommitSummary;

/// Minimal chip: `Akira · 3d`.
#[must_use]
pub fn format_minimal(commit: &CommitSummary, now: i64) -> String {
    let author = short_author(&commit.author);
    let age = relative_short(commit.timestamp, now);
    format!("{author} · {age}")
}

/// Hover detail: `Akira · 3 days ago · a182df · Optimize repository loading`.
#[must_use]
pub fn format_hover(commit: &CommitSummary, now: i64) -> String {
    let author = short_author(&commit.author);
    let age = relative_long(commit.timestamp, now);
    let short = short_oid(&commit.oid.0);
    format!("{author} · {age} · {short} · {}", commit.summary)
}

fn short_author(author: &str) -> String {
    let first = author.split_whitespace().next().unwrap_or(author);
    if first.len() > 16 {
        format!("{}…", &first[..15])
    } else {
        first.to_string()
    }
}

fn short_oid(oid: &str) -> String {
    if oid.len() > 7 {
        oid[..7].to_string()
    } else {
        oid.to_string()
    }
}

fn relative_short(timestamp: i64, now: i64) -> String {
    let days = days_ago(timestamp, now);
    match days {
        0 => "today".into(),
        1 => "1d".into(),
        d if d < 7 => format!("{d}d"),
        d if d < 30 => format!("{}w", d / 7),
        d if d < 365 => format!("{}mo", d / 30),
        d => format!("{}y", d / 365),
    }
}

fn relative_long(timestamp: i64, now: i64) -> String {
    let days = days_ago(timestamp, now);
    match days {
        0 => "today".into(),
        1 => "1 day ago".into(),
        d if d < 7 => format!("{d} days ago"),
        d if d < 30 => {
            let w = d / 7;
            if w == 1 {
                "1 week ago".into()
            } else {
                format!("{w} weeks ago")
            }
        }
        d if d < 365 => {
            let m = d / 30;
            if m == 1 {
                "1 month ago".into()
            } else {
                format!("{m} months ago")
            }
        }
        d => {
            let y = d / 365;
            if y == 1 {
                "1 year ago".into()
            } else {
                format!("{y} years ago")
            }
        }
    }
}

fn days_ago(timestamp: i64, now: i64) -> u64 {
    if now <= timestamp {
        return 0;
    }
    u64::try_from((now - timestamp) / 86_400).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::model::Oid;

    fn sample(ts: i64) -> CommitSummary {
        CommitSummary {
            oid: Oid("a182df0123456789".into()),
            summary: "Optimize repository loading".into(),
            author: "Akira Yamada".into(),
            timestamp: ts,
        }
    }

    #[test]
    fn minimal_and_hover_match_design() {
        let now = 1_700_000_000;
        let c = sample(now - 3 * 86_400);
        assert_eq!(format_minimal(&c, now), "Akira · 3d");
        assert_eq!(
            format_hover(&c, now),
            "Akira · 3 days ago · a182df0 · Optimize repository loading"
        );
    }
}
