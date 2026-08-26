//! Parse unified diff text into [`DiffContent`].

use crate::app::model::{DiffContent, DiffHunk, DiffLine, DiffTarget};

/// Parses a unified diff into app [`DiffContent`], tagging each line with its
/// body index (compatible with [`crate::git::patch::build_partial_patch`]).
#[must_use]
pub fn parse_diff_content(target: DiffTarget, text: &str) -> DiffContent {
    let mut hunks: Vec<DiffHunk> = Vec::new();
    let mut current: Option<DiffHunk> = None;
    let mut past_headers = false;
    let mut body_index = 0usize;

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
        if let Some(h) = current.as_mut() {
            h.lines.push(DiffLine {
                origin,
                content,
                body_index,
            });
        }
        body_index += 1;
    }
    if let Some(h) = current.take() {
        hunks.push(h);
    }

    DiffContent {
        target,
        hunks: hunks.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert_eq!(content.hunks[0].lines[0].body_index, 1); // after @@ at 0
        assert_eq!(content.hunks[0].lines[1].body_index, 2);
        assert_eq!(content.hunks[0].lines[1].origin, '+');
    }
}
