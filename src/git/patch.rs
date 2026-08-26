//! Partial unified-diff patch construction for line-level stage/unstage.
//!
//! See issue #28 and `docs/design/09-git-backend.md` (CLI `git apply --cached`).

use super::error::GitError;

/// One parsed line from a unified diff (excluding the `diff --git` preamble).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedDiffLine {
    /// Absolute index among body lines (hunk headers + content), 0-based.
    pub index: usize,
    pub kind: DiffLineKind,
    pub text: String,
}

/// Classification of a unified-diff body line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    HunkHeader,
    Context,
    Addition,
    Deletion,
}

/// Builds a minimal unified patch that includes only the selected addition /
/// deletion lines (plus required context and adjusted hunk headers).
///
/// `selected` are 0-based indices into the **body** of `unified_diff` (everything
/// after the `+++` / `---` headers), matching [`ParsedDiffLine::index`].
///
/// # Errors
/// Returns [`GitError::Backend`] when the diff cannot be parsed or no
/// stageable lines are selected.
pub fn build_partial_patch(unified_diff: &str, selected: &[usize]) -> Result<String, GitError> {
    if selected.is_empty() {
        return Err(GitError::Backend(
            "ステージする行が選択されていません".into(),
        ));
    }

    let (preamble, body) = split_preamble(unified_diff)?;
    let lines = parse_body(&body)?;
    let selected: std::collections::BTreeSet<usize> = selected.iter().copied().collect();

    // Keep only hunks that contain at least one selected +/- line.
    let mut out = preamble;
    if !out.ends_with('\n') {
        out.push('\n');
    }

    let hunks = split_hunks(&lines);
    let mut wrote_hunk = false;
    for hunk in hunks {
        if let Some(block) = filter_hunk(&hunk, &selected) {
            out.push_str(&block);
            wrote_hunk = true;
        }
    }

    if !wrote_hunk {
        return Err(GitError::Backend(
            "選択行から適用可能なパッチを作れませんでした".into(),
        ));
    }
    Ok(out)
}

fn split_preamble(diff: &str) -> Result<(String, String), GitError> {
    let mut preamble = String::new();
    let mut body = String::new();
    let mut in_body = false;
    for line in diff.lines() {
        if in_body {
            body.push_str(line);
            body.push('\n');
        } else {
            preamble.push_str(line);
            preamble.push('\n');
            if line.starts_with("+++ ") {
                in_body = true;
            }
        }
    }
    if !in_body {
        return Err(GitError::Backend("unified diff の形式が不正です".into()));
    }
    Ok((preamble, body))
}

fn parse_body(body: &str) -> Result<Vec<ParsedDiffLine>, GitError> {
    let mut lines = Vec::new();
    for (index, line) in body.lines().enumerate() {
        let kind = if line.starts_with("@@") {
            DiffLineKind::HunkHeader
        } else if line.starts_with('+') {
            DiffLineKind::Addition
        } else if line.starts_with('-') {
            DiffLineKind::Deletion
        } else if line.starts_with(' ') || line.is_empty() {
            DiffLineKind::Context
        } else if line.starts_with('\\') {
            // "\ No newline at end of file"
            DiffLineKind::Context
        } else {
            return Err(GitError::Backend(format!("不明な diff 行: {line}")));
        };
        lines.push(ParsedDiffLine {
            index,
            kind,
            text: line.to_string(),
        });
    }
    Ok(lines)
}

struct Hunk {
    old_start: u32,
    new_start: u32,
    lines: Vec<ParsedDiffLine>,
}

fn split_hunks(lines: &[ParsedDiffLine]) -> Vec<Hunk> {
    let mut hunks = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].kind != DiffLineKind::HunkHeader {
            i += 1;
            continue;
        }
        let header = lines[i].text.clone();
        let (old_start, new_start) = parse_hunk_starts(&header).unwrap_or((1, 1));
        i += 1;
        let mut body = Vec::new();
        while i < lines.len() && lines[i].kind != DiffLineKind::HunkHeader {
            body.push(lines[i].clone());
            i += 1;
        }
        hunks.push(Hunk {
            old_start,
            new_start,
            lines: body,
        });
    }
    hunks
}

fn parse_hunk_starts(header: &str) -> Option<(u32, u32)> {
    // @@ -l,s +l,s @@
    let rest = header.strip_prefix("@@")?;
    let mut parts = rest.split("@@");
    let ranges = parts.next()?.trim();
    let mut old_new = ranges.split_whitespace();
    let old = old_new.next()?.trim_start_matches('-');
    let new = old_new.next()?.trim_start_matches('+');
    let old_start = old.split(',').next()?.parse().ok()?;
    let new_start = new.split(',').next()?.parse().ok()?;
    Some((old_start, new_start))
}

fn filter_hunk(hunk: &Hunk, selected: &std::collections::BTreeSet<usize>) -> Option<String> {
    let has_selected = hunk.lines.iter().any(|l| {
        selected.contains(&l.index)
            && matches!(l.kind, DiffLineKind::Addition | DiffLineKind::Deletion)
    });
    if !has_selected {
        return None;
    }

    let mut old_count = 0u32;
    let mut new_count = 0u32;
    let mut body = String::new();

    for line in &hunk.lines {
        match line.kind {
            DiffLineKind::Context => {
                body.push_str(&line.text);
                body.push('\n');
                old_count += 1;
                new_count += 1;
            }
            DiffLineKind::Addition => {
                if selected.contains(&line.index) {
                    body.push_str(&line.text);
                    body.push('\n');
                    new_count += 1;
                }
                // Unselected additions are omitted (remain unstaged in worktree).
            }
            DiffLineKind::Deletion => {
                if selected.contains(&line.index) {
                    body.push_str(&line.text);
                    body.push('\n');
                    old_count += 1;
                } else {
                    // Unselected deletion must stay as context so the file still matches.
                    body.push(' ');
                    body.push_str(line.text.get(1..).unwrap_or(""));
                    body.push('\n');
                    old_count += 1;
                    new_count += 1;
                }
            }
            DiffLineKind::HunkHeader => {}
        }
    }

    if old_count == 0 && new_count == 0 {
        return None;
    }

    let header = format!(
        "@@ -{},{} +{},{} @@\n",
        hunk.old_start, old_count, hunk.new_start, new_count
    );
    let mut block = header;
    block.push_str(&body);
    Some(block)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
diff --git a/file.txt b/file.txt
index 1111111..2222222 100644
--- a/file.txt
+++ b/file.txt
@@ -1,2 +1,4 @@
 line1
-line2
+line2b
+line3
+line4
";

    #[test]
    fn selects_single_addition_line() {
        // Body indices: 0=@@, 1=' line1', 2='-line2', 3='+line2b', 4='+line3', 5='+line4'
        let patch = build_partial_patch(SAMPLE, &[4]).expect("patch");
        assert!(patch.contains("+line3"), "{patch}");
        assert!(!patch.contains("+line2b"), "{patch}");
        assert!(!patch.contains("+line4"), "{patch}");
        assert!(
            patch.contains("-line2") || patch.contains(" line2"),
            "{patch}"
        );
    }

    #[test]
    fn empty_selection_errors() {
        assert!(build_partial_patch(SAMPLE, &[]).is_err());
    }
}
