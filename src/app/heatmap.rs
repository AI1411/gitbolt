//! Blame Heatmap scoring (issue #33).
//!
//! Combines commit **recency** and **OID frequency** within the current diff
//! into a 0..=1 intensity for gutter coloring.

#![allow(clippy::cast_precision_loss)]

use std::collections::HashMap;

use crate::app::model::{CommitSummary, DiffContent, DiffLine};

/// Intensity in `0.0..=1.0` for a blamed line (higher = hotter / newer / more shared).
#[must_use]
pub fn intensity_for(commit: &CommitSummary, now: i64, oid_count: u32, max_count: u32) -> f32 {
    let recency = recency_score(commit.timestamp, now);
    let freq = if max_count == 0 {
        0.0
    } else {
        oid_count as f32 / max_count as f32
    };
    (0.65_f32 * recency + 0.35_f32 * freq).clamp(0.0, 1.0)
}

/// Maps intensity to a cool→warm CSS color (slate → amber → red).
#[must_use]
pub fn color_for(intensity: f32) -> &'static str {
    let t = intensity.clamp(0.0, 1.0);
    if t < 0.2 {
        "#1e3a5f"
    } else if t < 0.4 {
        "#0e7490"
    } else if t < 0.6 {
        "#ca8a04"
    } else if t < 0.8 {
        "#ea580c"
    } else {
        "#dc2626"
    }
}

/// Builds `body_index → intensity` for every line that has a change origin.
#[must_use]
pub fn intensities_for_diff(content: &DiffContent, now: i64) -> HashMap<usize, f32> {
    let mut oid_counts: HashMap<&str, u32> = HashMap::new();
    for line in iter_lines(content) {
        if let Some(origin) = line.change_origin.as_ref() {
            *oid_counts.entry(origin.oid.0.as_str()).or_insert(0) += 1;
        }
    }
    let max_count = oid_counts.values().copied().max().unwrap_or(0);
    let mut out = HashMap::new();
    for line in iter_lines(content) {
        if let Some(origin) = line.change_origin.as_ref() {
            let count = oid_counts.get(origin.oid.0.as_str()).copied().unwrap_or(0);
            out.insert(
                line.body_index,
                intensity_for(origin, now, count, max_count),
            );
        }
    }
    out
}

fn iter_lines(content: &DiffContent) -> impl Iterator<Item = &DiffLine> {
    content.hunks.iter().flat_map(|h| h.lines.iter())
}

fn recency_score(timestamp: i64, now: i64) -> f32 {
    let days = if now <= timestamp {
        0.0
    } else {
        ((now - timestamp) as f32) / 86_400.0
    };
    (1.0 - (days / 365.0).min(1.0)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::model::{DiffHunk, DiffTarget, Oid};
    use std::path::PathBuf;
    use std::sync::Arc;

    fn summary(oid: &str, ts: i64) -> CommitSummary {
        CommitSummary {
            oid: Oid(oid.into()),
            summary: "msg".into(),
            author: "A".into(),
            timestamp: ts,
        }
    }

    #[test]
    fn newer_commits_score_hotter() {
        let now = 1_700_000_000;
        let hot = intensity_for(&summary("a", now - 86_400), now, 1, 1);
        let cold = intensity_for(&summary("b", now - 400 * 86_400), now, 1, 1);
        assert!(hot > cold);
        assert!(hot > 0.5);
        assert!(cold < 0.4);
    }

    #[test]
    fn frequent_oid_boosts_intensity() {
        let now = 1_700_000_000;
        let alone = intensity_for(&summary("a", now - 30 * 86_400), now, 1, 10);
        let common = intensity_for(&summary("a", now - 30 * 86_400), now, 10, 10);
        assert!(common > alone);
    }

    #[test]
    fn color_bands_are_stable() {
        assert_eq!(color_for(0.0), "#1e3a5f");
        assert_eq!(color_for(0.5), "#ca8a04");
        assert_eq!(color_for(1.0), "#dc2626");
    }

    #[test]
    fn intensities_keyed_by_body_index() {
        let now = 1_700_000_000;
        let content = DiffContent {
            target: DiffTarget {
                path: PathBuf::from("a.rs"),
                staged: false,
            },
            hunks: Arc::from(vec![DiffHunk {
                header: "@@".into(),
                lines: vec![
                    DiffLine {
                        origin: ' ',
                        content: "a".into(),
                        body_index: 0,
                        old_line: Some(1),
                        new_line: Some(1),
                        change_origin: Some(summary("aaa", now)),
                    },
                    DiffLine {
                        origin: '+',
                        content: "b".into(),
                        body_index: 1,
                        old_line: None,
                        new_line: Some(2),
                        change_origin: None,
                    },
                ],
            }]),
            notice: None,
        };
        let map = intensities_for_diff(&content, now);
        assert!(map.contains_key(&0));
        assert!(!map.contains_key(&1));
        assert!(map[&0] > 0.9);
    }
}
