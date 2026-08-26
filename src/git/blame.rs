//! Blame at HEAD for Change Origin / Smart Blame (issues #31 / #22).

use std::collections::HashMap;
use std::path::Path;

use super::cli::GitCli;
use super::error::GitError;
use super::service::CommitInfo;

/// Returns blame entries for `path` as it exists at `HEAD`, indexed by
/// **1-based** line number. Missing path / no HEAD → empty map (not an error).
///
/// # Errors
/// Propagates CLI failures other than a missing path/object.
pub fn blame_at_head(repo: &Path, path: &Path) -> Result<HashMap<u32, CommitInfo>, GitError> {
    blame_lines(repo, path, &[])
}

/// Blames specific **1-based** lines (or the whole file when `lines` is empty).
///
/// Contiguous lines are coalesced into `git blame -L` ranges for large files.
///
/// # Errors
/// Propagates CLI failures other than a missing path/object.
pub fn blame_lines(
    repo: &Path,
    path: &Path,
    lines: &[u32],
) -> Result<HashMap<u32, CommitInfo>, GitError> {
    let cli = GitCli::new(repo)?;
    let rel = path.to_string_lossy();
    if lines.is_empty() {
        return run_blame(&cli, rel.as_ref(), None);
    }
    let mut sorted: Vec<u32> = lines.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    let mut map = HashMap::new();
    for (start, end) in coalesce_ranges(&sorted) {
        let range = format!("{start},{end}");
        let part = run_blame(&cli, rel.as_ref(), Some(&range))?;
        map.extend(part);
    }
    Ok(map)
}

fn run_blame(
    cli: &GitCli,
    rel: &str,
    range: Option<&str>,
) -> Result<HashMap<u32, CommitInfo>, GitError> {
    let out = match range {
        Some(r) => {
            let flag = format!("-L{r}");
            cli.run(&["blame", "--line-porcelain", &flag, "HEAD", "--", rel])
        }
        None => cli.run(&["blame", "--line-porcelain", "HEAD", "--", rel]),
    };
    let out = match out {
        Ok(o) => o,
        Err(GitError::Backend(msg))
            if msg.contains("no such path")
                || msg.contains("cannot find path")
                || msg.contains("does not exist")
                || msg.contains("no such ref")
                || msg.contains("bad object")
                || msg.contains("Not a valid object")
                || msg.contains("no such file") =>
        {
            return Ok(HashMap::new());
        }
        Err(e) => return Err(e),
    };
    Ok(parse_line_porcelain(&out))
}

/// Coalesce sorted unique line numbers into inclusive `(start, end)` ranges.
fn coalesce_ranges(sorted: &[u32]) -> Vec<(u32, u32)> {
    let mut ranges = Vec::new();
    let Some(&first) = sorted.first() else {
        return ranges;
    };
    let mut start = first;
    let mut prev = first;
    for &n in &sorted[1..] {
        if n == prev + 1 {
            prev = n;
            continue;
        }
        ranges.push((start, prev));
        start = n;
        prev = n;
    }
    ranges.push((start, prev));
    ranges
}

fn parse_line_porcelain(out: &str) -> HashMap<u32, CommitInfo> {
    let mut map = HashMap::new();
    let mut oid = String::new();
    let mut final_line: u32 = 0;
    let mut summary = String::new();
    let mut author = String::new();
    let mut time: i64 = 0;
    // Cache metadata for repeated commit hashes (porcelain omits fields after first).
    let mut meta: HashMap<String, (String, String, i64)> = HashMap::new();

    for line in out.lines() {
        if line.starts_with('\t') {
            if final_line > 0 && !oid.is_empty() {
                if let Some((s, a, t)) = meta.get(&oid) {
                    summary.clone_from(s);
                    author.clone_from(a);
                    time = *t;
                } else if !summary.is_empty() {
                    meta.insert(oid.clone(), (summary.clone(), author.clone(), time));
                }
                map.insert(
                    final_line,
                    CommitInfo {
                        oid: oid.clone(),
                        summary: summary.clone(),
                        author: author.clone(),
                        time,
                    },
                );
            }
            continue;
        }

        if line.len() >= 40
            && line.as_bytes()[..40].iter().all(u8::is_ascii_hexdigit)
            && line.as_bytes().get(40) == Some(&b' ')
        {
            let mut parts = line.split_whitespace();
            oid = parts.next().unwrap_or("").to_string();
            let _orig = parts.next();
            final_line = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            if !meta.contains_key(&oid) {
                summary.clear();
                author.clear();
                time = 0;
            }
            continue;
        }

        if let Some(v) = line.strip_prefix("summary ") {
            summary = v.to_string();
        } else if let Some(v) = line.strip_prefix("author ") {
            author = v.to_string();
        } else if let Some(v) = line.strip_prefix("author-time ") {
            time = v.parse().unwrap_or(0);
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::fixture::TempRepo;

    #[test]
    fn blame_maps_head_line_to_prior_commit() {
        let repo = TempRepo::init();
        repo.write("a.txt", "alpha\nbeta\ngamma\n");
        repo.stage("a.txt");
        repo.commit("seed");
        let seed = repo.run(&["rev-parse", "HEAD"]);

        repo.write("a.txt", "alpha\nBETA\ngamma\n");
        let blame = blame_at_head(repo.path(), Path::new("a.txt")).expect("blame");
        assert_eq!(blame.get(&2).map(|c| c.oid.as_str()), Some(seed.as_str()));
        assert!(blame[&2].summary.contains("seed"));
    }

    #[test]
    fn blame_missing_path_is_empty() {
        let repo = TempRepo::init();
        repo.write("keep.txt", "x\n");
        repo.stage("keep.txt");
        repo.commit("keep");
        let blame = blame_at_head(repo.path(), Path::new("never.txt")).expect("ok");
        assert!(blame.is_empty());
    }

    #[test]
    fn blame_lines_range_matches_full_for_subset() {
        let repo = TempRepo::init();
        repo.write("a.txt", "a\nb\nc\nd\ne\n");
        repo.stage("a.txt");
        repo.commit("all");
        let full = blame_at_head(repo.path(), Path::new("a.txt")).expect("full");
        let part = blame_lines(repo.path(), Path::new("a.txt"), &[2, 3, 5]).expect("part");
        assert_eq!(part.get(&2), full.get(&2));
        assert_eq!(part.get(&5), full.get(&5));
        assert!(!part.contains_key(&1));
    }

    #[test]
    fn coalesce_ranges_merges_contiguous() {
        assert_eq!(
            coalesce_ranges(&[1, 2, 3, 5, 7, 8]),
            vec![(1, 3), (5, 5), (7, 8)]
        );
    }
}
