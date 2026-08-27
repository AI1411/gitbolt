//! Shared visual design tokens (issue #108).
//!
//! Colors, radii, and type stacks live here as CSS custom properties so later
//! issues (status color, typography, light theme) can retune the UI in one place.

use crate::app::layout_prefs::ColorScheme;
use crate::app::model::ChangeKind;

/// Dark-theme custom properties applied on the application root.
pub const ROOT_VARS: &str = "\
--gb-bg:#0f1419;\
--gb-surface:#121820;\
--gb-surface-raised:#151b24;\
--gb-surface-mid:#1a2332;\
--gb-border:#243044;\
--gb-border-strong:#334155;\
--gb-text:#e8eef7;\
--gb-text-muted:#9fb0c7;\
--gb-text-faint:#94a3b8;\
--gb-text-disabled:#475569;\
--gb-accent:#3d8bfd;\
--gb-accent-muted:#93c5fd;\
--gb-selected:#1e3a5f;\
--gb-selected-row:#1a2740;\
--gb-chip:#1e293b;\
--gb-chip-text:#cbd5e1;\
--gb-chip-strong:#e2e8f0;\
--gb-danger:#fca5a5;\
--gb-danger-fg:#fecaca;\
--gb-danger-border:#7f1d1d;\
--gb-danger-bg:#1c1212;\
--gb-danger-strong:#b91c1c;\
--gb-warning:#fde68a;\
--gb-string:#fcd34d;\
--gb-link:#7dd3fc;\
--gb-add:#86efac;\
--gb-add-bg:#13281c;\
--gb-del:#fca5a5;\
--gb-del-bg:#2a1518;\
--gb-drop-idle:#4a5568;\
--gb-radius-sm:3px;\
--gb-radius:4px;\
--gb-radius-lg:6px;\
--gb-radius-xl:8px;\
--gb-radius-pill:999px;\
--gb-font:\"IBM Plex Sans\",\"Source Sans 3\",\"Segoe UI\",system-ui,-apple-system,sans-serif;\
--gb-mono:\"JetBrains Mono\",\"SF Mono\",\"Cascadia Code\",ui-monospace,Menlo,Consolas,monospace;\
--gb-size-hero:2rem;\
--gb-size-title:1.125rem;\
--gb-size-body:0.875rem;\
--gb-size-pulse:0.8125rem;\
--gb-size-label:0.6875rem;\
--gb-size-hint:0.75rem;\
--gb-size-diff:0.8125rem;\
--gb-weight-regular:500;\
--gb-weight-semibold:600;\
--gb-space-1:0.25rem;\
--gb-space-2:0.5rem;\
--gb-space-3:0.75rem;\
--gb-space-4:0.85rem;\
--gb-row:2.05rem;";

/// Light-theme custom properties (issue #118).
pub const LIGHT_VARS: &str = "\
--gb-bg:#f4f6fa;\
--gb-surface:#ffffff;\
--gb-surface-raised:#eef2f7;\
--gb-surface-mid:#e2e8f0;\
--gb-border:#d0d7e2;\
--gb-border-strong:#94a3b8;\
--gb-text:#1e293b;\
--gb-text-muted:#475569;\
--gb-text-faint:#64748b;\
--gb-text-disabled:#94a3b8;\
--gb-accent:#2563eb;\
--gb-accent-muted:#1d4ed8;\
--gb-selected:#dbeafe;\
--gb-selected-row:#e0e7ff;\
--gb-chip:#e2e8f0;\
--gb-chip-text:#334155;\
--gb-chip-strong:#1e293b;\
--gb-danger:#b91c1c;\
--gb-danger-fg:#7f1d1d;\
--gb-danger-border:#fecaca;\
--gb-danger-bg:#fef2f2;\
--gb-danger-strong:#dc2626;\
--gb-warning:#a16207;\
--gb-string:#b45309;\
--gb-link:#0369a1;\
--gb-add:#166534;\
--gb-add-bg:#dcfce7;\
--gb-del:#991b1b;\
--gb-del-bg:#fee2e2;\
--gb-drop-idle:#94a3b8;\
--gb-radius-sm:3px;\
--gb-radius:4px;\
--gb-radius-lg:6px;\
--gb-radius-xl:8px;\
--gb-radius-pill:999px;\
--gb-font:\"IBM Plex Sans\",\"Source Sans 3\",\"Segoe UI\",system-ui,-apple-system,sans-serif;\
--gb-mono:\"JetBrains Mono\",\"SF Mono\",\"Cascadia Code\",ui-monospace,Menlo,Consolas,monospace;\
--gb-size-hero:2rem;\
--gb-size-title:1.125rem;\
--gb-size-body:0.875rem;\
--gb-size-pulse:0.8125rem;\
--gb-size-label:0.6875rem;\
--gb-size-hint:0.75rem;\
--gb-size-diff:0.8125rem;\
--gb-weight-regular:500;\
--gb-weight-semibold:600;\
--gb-space-1:0.25rem;\
--gb-space-2:0.5rem;\
--gb-space-3:0.75rem;\
--gb-space-4:0.85rem;\
--gb-row:2.05rem;";

/// Global stylesheet (focus rings and later chrome). Injected once on the app root.
pub const GLOBAL_CSS: &str = r"
.gb-selectable:focus-visible{outline:2px solid var(--gb-accent);outline-offset:-2px;}
.gb-selectable{border-radius:var(--gb-radius);}
.nav-item{position:relative;}
.nav-item.active{box-shadow:inset 3px 0 0 var(--gb-accent);}
.resize-handle{flex:0 0 5px;cursor:col-resize;background:transparent;}
.resize-handle:hover,.resize-handle:active{background:var(--gb-accent);opacity:0.45;}
.gb-hunk-header{position:sticky;top:0;z-index:2;backdrop-filter:blur(6px);}
.gb-diff-scroll{overflow:auto;min-height:0;}
.gb-diff-line{display:flex;align-items:stretch;gap:0.35rem;padding:0.08rem 0.55rem 0.08rem 0;white-space:pre;line-height:1.45;}
.gb-diff-line.add{box-shadow:inset 3px 0 0 var(--gb-add);}
.gb-diff-line.del{box-shadow:inset 3px 0 0 var(--gb-del);}
.gb-lineno{flex:0 0 3.5ch;text-align:right;opacity:0.4;font-size:0.72em;user-select:none;align-self:baseline;padding-top:0.1em;font-variant-numeric:tabular-nums;}
.gb-graph{width:14px;height:100%;min-height:2.05rem;display:block;}
";

/// Inline style for the application root (`:root` equivalent for the window).
#[must_use]
pub fn root_style(scheme: ColorScheme) -> String {
    let vars = match scheme {
        ColorScheme::Dark => ROOT_VARS,
        ColorScheme::Light => LIGHT_VARS,
    };
    format!(
        "{vars}width:100vw;height:100vh;margin:0;background:var(--gb-bg);\
         color:var(--gb-text);font-family:var(--gb-font);"
    )
}

/// Selected-row background token (or transparent).
#[must_use]
pub fn selected_bg(on: bool) -> &'static str {
    if on {
        "var(--gb-selected)"
    } else {
        "transparent"
    }
}

/// Background + keyboard-cursor accent for a selected row.
#[must_use]
pub fn row_style(selected: bool) -> String {
    if selected {
        "background:var(--gb-selected);box-shadow:inset 2px 0 0 var(--gb-accent);".into()
    } else {
        "background:transparent;box-shadow:none;".into()
    }
}

/// Foreground color token for a file-status mark (`A` / `M` / `D` / `?` / …).
#[must_use]
pub fn status_fg(kind: ChangeKind) -> &'static str {
    match kind {
        ChangeKind::Added | ChangeKind::Copied => "var(--gb-add)",
        ChangeKind::Modified | ChangeKind::Renamed | ChangeKind::TypeChanged => "var(--gb-warning)",
        ChangeKind::Deleted => "var(--gb-del)",
        ChangeKind::Untracked => "var(--gb-link)",
        ChangeKind::Conflicted => "var(--gb-danger)",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_style_declares_core_tokens() {
        let css = root_style(ColorScheme::Dark);
        for token in [
            "--gb-bg:",
            "--gb-surface:",
            "--gb-text:",
            "--gb-accent:",
            "--gb-selected:",
            "--gb-radius:",
            "--gb-font:",
            "--gb-mono:",
            "--gb-size-title:",
            "--gb-size-hint:",
            "--gb-size-diff:",
            "--gb-mono:",
        ] {
            assert!(css.contains(token), "missing {token}");
        }
        assert!(GLOBAL_CSS.contains(".nav-item.active"));
        assert!(GLOBAL_CSS.contains(".resize-handle:hover"));
        assert!(GLOBAL_CSS.contains(".gb-hunk-header"));
        assert!(GLOBAL_CSS.contains(".gb-diff-line.add"));
        let light = root_style(ColorScheme::Light);
        assert!(light.contains("--gb-bg:#f4f6fa"));
        assert_ne!(light, root_style(ColorScheme::Dark));
    }

    #[test]
    fn selected_bg_toggles() {
        assert_eq!(selected_bg(true), "var(--gb-selected)");
        assert_eq!(selected_bg(false), "transparent");
        assert!(row_style(true).contains("box-shadow:inset 2px 0 0"));
        assert!(row_style(false).contains("box-shadow:none"));
    }

    #[test]
    fn status_fg_distinguishes_kinds() {
        assert_eq!(status_fg(ChangeKind::Added), "var(--gb-add)");
        assert_eq!(status_fg(ChangeKind::Deleted), "var(--gb-del)");
        assert_eq!(status_fg(ChangeKind::Modified), "var(--gb-warning)");
        assert_eq!(status_fg(ChangeKind::Untracked), "var(--gb-link)");
        assert_eq!(status_fg(ChangeKind::Conflicted), "var(--gb-danger)");
    }
}
