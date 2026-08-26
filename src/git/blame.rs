//! Blame at HEAD for Change Origin (issue #31).

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
    let cli = GitCli::new(repo)?;
    let rel = path.to_string_lossy();
    let out = match cli.run(&["blame", "--line-porcelain", "HEAD", "--", rel.as_ref()]) {
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
            // Content line — ignore body text.
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
            // Reset per-block fields; may be filled from meta cache on content line.
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
}
