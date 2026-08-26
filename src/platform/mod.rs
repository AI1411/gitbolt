//! Platform-specific adapters (macOS-first MVP).

/// Target platform identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    MacOs,
    Windows,
    Linux,
}

/// Returns the current runtime platform.
#[must_use]
pub fn current_platform() -> Platform {
    if cfg!(target_os = "macos") {
        Platform::MacOs
    } else if cfg!(target_os = "windows") {
        Platform::Windows
    } else {
        Platform::Linux
    }
}

/// Modifier key glyph for shortcut hints (`⌘` on macOS, `Ctrl` elsewhere).
#[must_use]
pub fn mod_key_label() -> &'static str {
    match current_platform() {
        Platform::MacOs => "⌘",
        Platform::Windows | Platform::Linux => "Ctrl",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mod_key_label_is_nonempty() {
        assert!(!mod_key_label().is_empty());
    }
}
