//! Detect Issue / PR references in commit messages and branch names (issue #76).

use crate::git::remote_link::{RemoteWeb, WebHostKind};

/// Kind of tracked reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IssueKind {
    Issue,
    PullRequest,
}

/// A detected issue or pull-request reference.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IssueRef {
    pub kind: IssueKind,
    pub number: u32,
    pub raw: String,
}

/// Scans `text` for `#123`, `GH-123`, and full host issue/PR URLs.
#[must_use]
pub fn detect_issue_refs(text: &str) -> Vec<IssueRef> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();

    push_url_refs(text, &mut out, &mut seen);
    push_gh_prefix_refs(text, &mut out, &mut seen);
    push_hash_refs(text, &mut out, &mut seen);

    out
}

/// Builds a browser URL for `r` on `web`, if the host is known.
#[must_use]
pub fn resolve_issue_url(web: &RemoteWeb, r: &IssueRef) -> Option<String> {
    match web.kind {
        WebHostKind::GitHub | WebHostKind::Unknown => Some(match r.kind {
            IssueKind::Issue => format!("{}/issues/{}", web.web_base, r.number),
            IssueKind::PullRequest => format!("{}/pull/{}", web.web_base, r.number),
        }),
        WebHostKind::GitLab => Some(match r.kind {
            IssueKind::Issue => format!("{}/-/issues/{}", web.web_base, r.number),
            IssueKind::PullRequest => format!("{}/-/merge_requests/{}", web.web_base, r.number),
        }),
        WebHostKind::Bitbucket => Some(match r.kind {
            IssueKind::Issue => format!("{}/issues/{}", web.web_base, r.number),
            IssueKind::PullRequest => format!("{}/pull-requests/{}", web.web_base, r.number),
        }),
    }
}

fn push_unique(
    out: &mut Vec<IssueRef>,
    seen: &mut std::collections::HashSet<(IssueKind, u32)>,
    r: IssueRef,
) {
    if seen.insert((r.kind, r.number)) {
        out.push(r);
    }
}

fn push_url_refs(
    text: &str,
    out: &mut Vec<IssueRef>,
    seen: &mut std::collections::HashSet<(IssueKind, u32)>,
) {
    // Patterns: /issues/N, /pull/N, /-/issues/N, /-/merge_requests/N, /pull-requests/N
    for (needle, kind) in [
        ("/issues/", IssueKind::Issue),
        ("/-/issues/", IssueKind::Issue),
        ("/pull/", IssueKind::PullRequest),
        ("/-/merge_requests/", IssueKind::PullRequest),
        ("/pull-requests/", IssueKind::PullRequest),
    ] {
        let mut start = 0;
        while let Some(rel) = text[start..].find(needle) {
            let at = start + rel + needle.len();
            if let Some((n, end)) = parse_u32_prefix(&text[at..]) {
                let raw_start = find_url_start(text, start + rel);
                let raw = text[raw_start..at + end].to_string();
                push_unique(
                    out,
                    seen,
                    IssueRef {
                        kind,
                        number: n,
                        raw,
                    },
                );
                start = at + end;
            } else {
                start = at;
            }
        }
    }
}

fn find_url_start(text: &str, marker: usize) -> usize {
    text[..marker]
        .rfind("http://")
        .or_else(|| text[..marker].rfind("https://"))
        .unwrap_or(marker)
}

fn push_gh_prefix_refs(
    text: &str,
    out: &mut Vec<IssueRef>,
    seen: &mut std::collections::HashSet<(IssueKind, u32)>,
) {
    let lower = text.to_ascii_lowercase();
    let mut start = 0;
    while let Some(rel) = lower[start..].find("gh-") {
        let at = start + rel;
        let ok_before = at == 0
            || !text
                .as_bytes()
                .get(at - 1)
                .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_');
        if ok_before {
            let num_at = at + 3;
            if let Some((n, end)) = parse_u32_prefix(&text[num_at..]) {
                push_unique(
                    out,
                    seen,
                    IssueRef {
                        kind: IssueKind::Issue,
                        number: n,
                        raw: text[at..num_at + end].to_string(),
                    },
                );
                start = num_at + end;
                continue;
            }
        }
        start = at + 3;
    }
}

fn push_hash_refs(
    text: &str,
    out: &mut Vec<IssueRef>,
    seen: &mut std::collections::HashSet<(IssueKind, u32)>,
) {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'#' {
            let ok_before = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
            if ok_before {
                if let Some((n, end)) = parse_u32_prefix(&text[i + 1..]) {
                    // Skip pure hex-looking long hashes after # (unlikely) — require reasonable issue size.
                    if end <= 8 {
                        push_unique(
                            out,
                            seen,
                            IssueRef {
                                kind: IssueKind::Issue,
                                number: n,
                                raw: text[i..i + 1 + end].to_string(),
                            },
                        );
                    }
                    i += 1 + end;
                    continue;
                }
            }
        }
        i += 1;
    }
}

fn parse_u32_prefix(s: &str) -> Option<(u32, usize)> {
    let mut end = 0;
    for (i, c) in s.char_indices() {
        if c.is_ascii_digit() {
            end = i + 1;
        } else {
            break;
        }
    }
    if end == 0 {
        return None;
    }
    let n: u32 = s[..end].parse().ok()?;
    Some((n, end))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::remote_link::parse_remote_url;

    #[test]
    fn detects_hash_and_gh_prefix() {
        let refs = detect_issue_refs("fix: crash (#76) and GH-12 leftover");
        assert!(refs.iter().any(|r| r.number == 76 && r.raw == "#76"));
        assert!(refs
            .iter()
            .any(|r| r.number == 12 && r.raw.eq_ignore_ascii_case("GH-12")));
    }

    #[test]
    fn detects_github_urls() {
        let refs = detect_issue_refs(
            "see https://github.com/ai1411/gitbolt/pull/91 and /issues/75 elsewhere https://github.com/ai1411/gitbolt/issues/75",
        );
        assert!(refs
            .iter()
            .any(|r| r.kind == IssueKind::PullRequest && r.number == 91));
        assert!(refs
            .iter()
            .any(|r| r.kind == IssueKind::Issue && r.number == 75));
    }

    #[test]
    fn resolves_github_urls() {
        let web = parse_remote_url("https://github.com/ai1411/gitbolt.git").unwrap();
        let issue = IssueRef {
            kind: IssueKind::Issue,
            number: 76,
            raw: "#76".into(),
        };
        assert_eq!(
            resolve_issue_url(&web, &issue).as_deref(),
            Some("https://github.com/ai1411/gitbolt/issues/76")
        );
        let pr = IssueRef {
            kind: IssueKind::PullRequest,
            number: 91,
            raw: "pull/91".into(),
        };
        assert_eq!(
            resolve_issue_url(&web, &pr).as_deref(),
            Some("https://github.com/ai1411/gitbolt/pull/91")
        );
    }

    #[test]
    fn dedupes_same_number() {
        let refs = detect_issue_refs("#76 and again #76");
        assert_eq!(refs.iter().filter(|r| r.number == 76).count(), 1);
    }
}
