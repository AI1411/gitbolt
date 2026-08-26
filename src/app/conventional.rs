//! Conventional Commits helpers for `CommitBox` completion (issue #77).

/// Standard Conventional Commit types.
pub const TYPES: &[&str] = &[
    "feat", "fix", "docs", "style", "refactor", "perf", "test", "build", "ci", "chore", "revert",
];

/// Returns type suggestions matching the leading token of `message`.
#[must_use]
pub fn suggest_types(message: &str) -> Vec<&'static str> {
    let head = message.split(['(', ':', ' ', '\n']).next().unwrap_or("");
    if head.is_empty() {
        return TYPES.to_vec();
    }
    // Already completed a type with `:` — no type suggestions.
    if message.contains(':') {
        return Vec::new();
    }
    TYPES
        .iter()
        .copied()
        .filter(|t| t.starts_with(head) && *t != head)
        .collect()
}

/// Whether the message already has `type:` or `type(scope):`.
#[must_use]
pub fn has_type_prefix(message: &str) -> bool {
    let Some((head, _)) = message.split_once(':') else {
        return false;
    };
    let ty = head.split('(').next().unwrap_or(head).trim();
    TYPES.contains(&ty)
}

/// Suggest scopes from changed file paths (top-level dir under `src/` or first segment).
#[must_use]
pub fn suggest_scopes(message: &str, paths: &[impl AsRef<std::path::Path>]) -> Vec<String> {
    let trimmed = message.trim();
    let type_only = TYPES.contains(&trimmed);
    let open_paren = trimmed.contains('(') && !trimmed.contains(')');
    if !type_only && !open_paren {
        return Vec::new();
    }

    let mut scopes = Vec::new();
    for p in paths {
        let p = p.as_ref();
        let mut comps = p.components().filter_map(|c| match c {
            std::path::Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            _ => None,
        });
        let scope = match (comps.next(), comps.next()) {
            (Some(first), Some(second)) if first == "src" => second,
            (Some(first), _) => first,
            _ => continue,
        };
        if !scopes.iter().any(|x| x == &scope) {
            scopes.push(scope);
        }
        if scopes.len() >= 8 {
            break;
        }
    }
    scopes
}

/// Inserts or replaces the type prefix, preserving any subject after `:`.
#[must_use]
pub fn apply_type(message: &str, ty: &str) -> String {
    if let Some((_, rest)) = message.split_once(':') {
        let rest = rest.trim_start();
        if rest.is_empty() {
            format!("{ty}: ")
        } else {
            format!("{ty}: {rest}")
        }
    } else if message.contains('(') {
        // type(scope partial
        if let Some((_, after)) = message.split_once('(') {
            format!("{ty}({after}")
        } else {
            format!("{ty}: ")
        }
    } else {
        format!("{ty}: ")
    }
}

/// Applies a scope into `type(scope): subject` form.
#[must_use]
pub fn apply_scope(message: &str, scope: &str) -> String {
    let ty = TYPES
        .iter()
        .find(|t| message.starts_with(*t))
        .copied()
        .unwrap_or("feat");
    let subject = message
        .split_once(':')
        .map(|(_, r)| r.trim_start().to_string())
        .unwrap_or_default();
    if subject.is_empty() {
        format!("{ty}({scope}): ")
    } else {
        format!("{ty}({scope}): {subject}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn suggest_types_filters_prefix() {
        assert!(suggest_types("").contains(&"feat"));
        assert_eq!(suggest_types("fe"), vec!["feat"]);
        assert!(suggest_types("feat: done").is_empty());
    }

    #[test]
    fn apply_type_preserves_subject() {
        assert_eq!(apply_type("", "fix"), "fix: ");
        assert_eq!(apply_type("feat: hello", "fix"), "fix: hello");
    }

    #[test]
    fn apply_scope_builds_header() {
        assert_eq!(apply_scope("feat", "ui"), "feat(ui): ");
        assert_eq!(apply_scope("fix: bug", "git"), "fix(git): bug");
    }

    #[test]
    fn suggest_scopes_from_paths() {
        let paths = [
            PathBuf::from("src/ui/context.rs"),
            PathBuf::from("src/git/remote.rs"),
        ];
        let scopes = suggest_scopes("feat", &paths);
        assert!(scopes.iter().any(|s| s == "ui"));
        assert!(scopes.iter().any(|s| s == "git"));
    }
}
