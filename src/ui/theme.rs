//! Shared visual design tokens (issue #108).
//!
//! Colors, radii, and type stacks live here as CSS custom properties so later
//! issues (status color, typography, light theme) can retune the UI in one place.

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
--gb-font:ui-sans-serif,system-ui,sans-serif;\
--gb-mono:ui-monospace,SFMono-Regular,Menlo,monospace;\
--gb-space-1:0.25rem;\
--gb-space-2:0.5rem;\
--gb-space-3:0.75rem;\
--gb-space-4:0.85rem;";

/// Inline style for the application root (`:root` equivalent for the window).
#[must_use]
pub fn root_style() -> String {
    format!(
        "{ROOT_VARS}width:100vw;height:100vh;margin:0;background:var(--gb-bg);\
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_style_declares_core_tokens() {
        let css = root_style();
        for token in [
            "--gb-bg:",
            "--gb-surface:",
            "--gb-text:",
            "--gb-accent:",
            "--gb-selected:",
            "--gb-radius:",
            "--gb-font:",
        ] {
            assert!(css.contains(token), "missing {token}");
        }
    }

    #[test]
    fn selected_bg_toggles() {
        assert_eq!(selected_bg(true), "var(--gb-selected)");
        assert_eq!(selected_bg(false), "transparent");
    }
}
