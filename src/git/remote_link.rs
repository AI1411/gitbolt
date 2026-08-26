//! Resolve git remote URLs into browser commit / file / line links (issue #75).
//!
//! Host detection is local (no API): GitHub, GitLab, Bitbucket, or a generic
//! GitHub-shaped fallback for unknown HTTPS remotes.

/// Recognized web host family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebHostKind {
    GitHub,
    GitLab,
    Bitbucket,
    /// HTTPS remote on an unknown host — use GitHub-like path shapes.
    Unknown,
}

/// Parsed remote suitable for building web URLs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteWeb {
    pub kind: WebHostKind,
    /// Origin without `.git`, e.g. `https://github.com/owner/repo`.
    pub web_base: String,
}

/// Parses a git remote URL into a [`RemoteWeb`].
///
/// Returns `None` for local paths, `file://` URLs, or unparseable input.
#[must_use]
pub fn parse_remote_url(url: &str) -> Option<RemoteWeb> {
    let url = url.trim();
    if url.is_empty() {
        return None;
    }
    if url.starts_with('/') || url.starts_with("file://") {
        return None;
    }

    let (host, path) = if let Some(rest) = url.strip_prefix("git@") {
        // git@host:owner/repo.git
        let (host, path) = rest.split_once(':')?;
        (host.to_string(), path.to_string())
    } else if let Some(rest) = url
        .strip_prefix("ssh://git@")
        .or_else(|| url.strip_prefix("ssh://"))
    {
        // ssh://git@host/owner/repo.git  or ssh://host/owner/repo
        let rest = rest.trim_start_matches('/');
        let (host, path) = rest.split_once('/')?;
        // host may include port: host:22
        let host = host.split(':').next().unwrap_or(host);
        (host.to_string(), path.to_string())
    } else if let Some(rest) = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
    {
        let scheme = if url.starts_with("https://") {
            "https"
        } else {
            "http"
        };
        let rest = rest.trim_start_matches('/');
        let (host, path) = rest.split_once('/')?;
        let host = host.split(':').next().unwrap_or(host);
        let path = path.trim_end_matches('/').trim_end_matches(".git");
        if path.is_empty() {
            return None;
        }
        let kind = classify_host(host);
        return Some(RemoteWeb {
            kind,
            web_base: format!("{scheme}://{host}/{path}"),
        });
    } else {
        return None;
    };

    let path = path.trim_end_matches('/').trim_end_matches(".git");
    if path.is_empty() {
        return None;
    }
    let kind = classify_host(&host);
    Some(RemoteWeb {
        kind,
        web_base: format!("https://{host}/{path}"),
    })
}

fn classify_host(host: &str) -> WebHostKind {
    let h = host.to_ascii_lowercase();
    if h == "github.com" || h.ends_with(".github.com") {
        WebHostKind::GitHub
    } else if h == "gitlab.com" || h.contains("gitlab") {
        WebHostKind::GitLab
    } else if h == "bitbucket.org" || h.contains("bitbucket") {
        WebHostKind::Bitbucket
    } else {
        WebHostKind::Unknown
    }
}

/// Commit page URL for `oid`.
#[must_use]
pub fn commit_url(web: &RemoteWeb, oid: &str) -> String {
    match web.kind {
        WebHostKind::GitLab => format!("{}/-/commit/{oid}", web.web_base),
        WebHostKind::Bitbucket => format!("{}/commits/{oid}", web.web_base),
        WebHostKind::GitHub | WebHostKind::Unknown => {
            format!("{}/commit/{oid}", web.web_base)
        }
    }
}

/// File-at-revision URL (`rev` may be a branch or oid).
#[must_use]
pub fn file_url(web: &RemoteWeb, rev: &str, path: &str) -> String {
    let path = path.trim_start_matches('/');
    match web.kind {
        WebHostKind::GitLab => format!("{}/-/blob/{rev}/{path}", web.web_base),
        WebHostKind::Bitbucket => format!("{}/src/{rev}/{path}", web.web_base),
        WebHostKind::GitHub | WebHostKind::Unknown => {
            format!("{}/blob/{rev}/{path}", web.web_base)
        }
    }
}

/// File + line URL.
#[must_use]
pub fn line_url(web: &RemoteWeb, rev: &str, path: &str, line: u32) -> String {
    let base = file_url(web, rev, path);
    match web.kind {
        WebHostKind::Bitbucket => format!("{base}#lines-{line}"),
        WebHostKind::GitHub | WebHostKind::GitLab | WebHostKind::Unknown => {
            format!("{base}#L{line}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_https_github() {
        let w = parse_remote_url("https://github.com/ai1411/gitbolt.git").unwrap();
        assert_eq!(w.kind, WebHostKind::GitHub);
        assert_eq!(w.web_base, "https://github.com/ai1411/gitbolt");
        assert_eq!(
            commit_url(&w, "abc123"),
            "https://github.com/ai1411/gitbolt/commit/abc123"
        );
        assert_eq!(
            file_url(&w, "abc123", "src/main.rs"),
            "https://github.com/ai1411/gitbolt/blob/abc123/src/main.rs"
        );
        assert_eq!(
            line_url(&w, "abc123", "src/main.rs", 10),
            "https://github.com/ai1411/gitbolt/blob/abc123/src/main.rs#L10"
        );
    }

    #[test]
    fn parses_ssh_github() {
        let w = parse_remote_url("git@github.com:ai1411/gitbolt.git").unwrap();
        assert_eq!(w.web_base, "https://github.com/ai1411/gitbolt");
    }

    #[test]
    fn parses_gitlab_https() {
        let w = parse_remote_url("https://gitlab.com/group/proj.git").unwrap();
        assert_eq!(w.kind, WebHostKind::GitLab);
        assert_eq!(
            commit_url(&w, "deadbeef"),
            "https://gitlab.com/group/proj/-/commit/deadbeef"
        );
        assert_eq!(
            line_url(&w, "main", "a.rs", 3),
            "https://gitlab.com/group/proj/-/blob/main/a.rs#L3"
        );
    }

    #[test]
    fn parses_bitbucket() {
        let w = parse_remote_url("https://bitbucket.org/team/repo.git").unwrap();
        assert_eq!(w.kind, WebHostKind::Bitbucket);
        assert_eq!(
            commit_url(&w, "abc"),
            "https://bitbucket.org/team/repo/commits/abc"
        );
        assert_eq!(
            line_url(&w, "abc", "f.rs", 7),
            "https://bitbucket.org/team/repo/src/abc/f.rs#lines-7"
        );
    }

    #[test]
    fn rejects_local_path() {
        assert!(parse_remote_url("/tmp/bare.git").is_none());
        assert!(parse_remote_url("file:///tmp/bare.git").is_none());
    }

    #[test]
    fn parses_ssh_url_scheme() {
        let w = parse_remote_url("ssh://git@github.com/ai1411/gitbolt.git").unwrap();
        assert_eq!(w.web_base, "https://github.com/ai1411/gitbolt");
    }

    #[test]
    fn parses_nested_gitlab_group() {
        let w = parse_remote_url("git@gitlab.com:org/team/proj.git").unwrap();
        assert_eq!(w.kind, WebHostKind::GitLab);
        assert_eq!(w.web_base, "https://gitlab.com/org/team/proj");
    }
}
