//! Parse unified diff text into [`DiffContent`].

use crate::app::model::{CommitSummary, DiffContent, DiffHunk, DiffLine, DiffTarget, Oid};
use crate::git::CommitInfo;

/// Max unified-diff characters before we truncate the view (issue #13).
pub const MAX_DIFF_CHARS: usize = 512_000;

/// Parses a unified diff into app [`DiffContent`], tagging each line with its
/// body index (compatible with [`crate::git::patch::build_partial_patch`]) and
/// old-side line numbers for Change Origin.
#[must_use]
pub fn parse_diff_content(target: DiffTarget, text: &str) -> DiffContent {
    if text.contains('\0') {
        return DiffContent {
            target,
            hunks: std::sync::Arc::from([] as [DiffHunk; 0]),
            notice: Some("Binary file (diff omitted)".into()),
        };
    }

    let mut notice = None;
    let text = if text.len() > MAX_DIFF_CHARS {
        notice = Some(format!(
            "Diff truncated to first {MAX_DIFF_CHARS} bytes (file too large)"
        ));
        &text[..MAX_DIFF_CHARS]
    } else {
        text
    };

    let mut hunks: Vec<DiffHunk> = Vec::new();
    let mut current: Option<DiffHunk> = None;
    let mut past_headers = false;
    let mut body_index = 0usize;
    let mut old_line: u32 = 0;
    let mut new_line: u32 = 0;

    for line in text.lines() {
        if !past_headers {
            if line.starts_with("+++ ") {
                past_headers = true;
            }
            continue;
        }

        if line.starts_with("@@") {
            if let Some(h) = current.take() {
                hunks.push(h);
            }
            old_line = parse_old_start(line).unwrap_or(0);
            new_line = parse_new_start(line).unwrap_or(0);
            current = Some(DiffHunk {
                header: line.to_string(),
                lines: Vec::new(),
            });
            body_index += 1;
            continue;
        }

        let origin = line.chars().next().unwrap_or(' ');
        let content = if line.is_empty() {
            String::new()
        } else {
            line[1..].to_string()
        };
        let line_old = match origin {
            '+' => None,
            _ if old_line > 0 => Some(old_line),
            _ => None,
        };
        let line_new = match origin {
            '-' => None,
            _ if new_line > 0 => Some(new_line),
            _ => None,
        };
        if origin != '+' && old_line > 0 {
            old_line += 1;
        }
        if origin != '-' && new_line > 0 {
            new_line += 1;
        }
        if let Some(h) = current.as_mut() {
            h.lines.push(DiffLine {
                origin,
                content,
                body_index,
                old_line: line_old,
                new_line: line_new,
                change_origin: None,
            });
        }
        body_index += 1;
    }
    if let Some(h) = current.take() {
        hunks.push(h);
    }

    if hunks.is_empty() && notice.is_none() {
        notice = Some("No textual diff".into());
    }

    DiffContent {
        target,
        hunks: hunks.into(),
        notice,
    }
}

/// Attaches HEAD blame commits onto lines that have an old-side line number.
#[must_use]
pub fn attach_change_origins(
    mut content: DiffContent,
    blame: &std::collections::HashMap<u32, CommitInfo, impl std::hash::BuildHasher>,
) -> DiffContent {
    let hunks: Vec<DiffHunk> = content
        .hunks
        .iter()
        .map(|h| DiffHunk {
            header: h.header.clone(),
            lines: h
                .lines
                .iter()
                .map(|line| {
                    let change_origin = line.old_line.and_then(|n| blame.get(&n)).map(to_summary);
                    DiffLine {
                        origin: line.origin,
                        content: line.content.clone(),
                        body_index: line.body_index,
                        old_line: line.old_line,
                        new_line: line.new_line,
                        change_origin,
                    }
                })
                .collect(),
        })
        .collect();
    content.hunks = hunks.into();
    content
}

fn to_summary(c: &CommitInfo) -> CommitSummary {
    CommitSummary {
        oid: Oid(c.oid.clone()),
        summary: c.summary.clone(),
        author: c.author.clone(),
        timestamp: c.time,
    }
}

/// Parses the old-file start from `@@ -12,3 +14,4 @@` → 12.
fn parse_old_start(header: &str) -> Option<u32> {
    let rest = header.strip_prefix("@@")?;
    let minus = rest.find('-')?;
    let after = &rest[minus + 1..];
    let end = after.find([',', ' ', '+']).unwrap_or(after.len());
    after[..end].parse().ok()
}

/// Parses the new-file start from `@@ -12,3 +14,4 @@` → 14.
fn parse_new_start(header: &str) -> Option<u32> {
    let plus = header.rfind('+')?;
    let after = &header[plus + 1..];
    let end = after.find([',', ' ']).unwrap_or(after.len());
    after[..end].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn body_index_skips_preamble_and_counts_headers() {
        let text = "\
diff --git a/a.txt b/a.txt
--- a/a.txt
+++ b/a.txt
@@ -1 +1,2 @@
 one
+two
";
        let content = parse_diff_content(
            DiffTarget {
                path: PathBuf::from("a.txt"),
                staged: false,
            },
            text,
        );
        assert_eq!(content.hunks[0].lines[0].body_index, 1);
        assert_eq!(content.hunks[0].lines[0].old_line, Some(1));
        assert_eq!(content.hunks[0].lines[0].new_line, Some(1));
        assert_eq!(content.hunks[0].lines[1].body_index, 2);
        assert_eq!(content.hunks[0].lines[1].origin, '+');
        assert_eq!(content.hunks[0].lines[1].old_line, None);
        assert_eq!(content.hunks[0].lines[1].new_line, Some(2));
    }

    #[test]
    fn dual_line_numbers_on_replace() {
        let text = "\
diff --git a/a.txt b/a.txt
--- a/a.txt
+++ b/a.txt
@@ -2 +2 @@
-beta
+BETA
";
        let content = parse_diff_content(
            DiffTarget {
                path: PathBuf::from("a.txt"),
                staged: false,
            },
            text,
        );
        assert_eq!(content.hunks[0].lines[0].old_line, Some(2));
        assert_eq!(content.hunks[0].lines[0].new_line, None);
        assert_eq!(content.hunks[0].lines[1].old_line, None);
        assert_eq!(content.hunks[0].lines[1].new_line, Some(2));
    }

    #[test]
    fn binary_diff_sets_notice() {
        let content = parse_diff_content(
            DiffTarget {
                path: PathBuf::from("x.bin"),
                staged: false,
            },
            "binary\0data",
        );
        assert!(content.hunks.is_empty());
        assert!(content.notice.as_deref().unwrap().contains("Binary"));
    }

    #[test]
    fn attach_origins_joins_blame_on_old_lines() {
        let text = "\
diff --git a/a.txt b/a.txt
--- a/a.txt
+++ b/a.txt
@@ -2 +2 @@
-beta
+BETA
";
        let content = parse_diff_content(
            DiffTarget {
                path: PathBuf::from("a.txt"),
                staged: false,
            },
            text,
        );
        assert_eq!(content.hunks[0].lines[0].old_line, Some(2));
        let mut blame = HashMap::new();
        blame.insert(
            2,
            CommitInfo {
                oid: "abc".into(),
                summary: "seed".into(),
                author: "T".into(),
                time: 1,
            },
        );
        let joined = attach_change_origins(content, &blame);
        let origin = joined.hunks[0].lines[0]
            .change_origin
            .as_ref()
            .expect("origin");
        assert_eq!(origin.oid.0, "abc");
        assert_eq!(origin.summary, "seed");
        assert!(joined.hunks[0].lines[1].change_origin.is_none());
    }
}
