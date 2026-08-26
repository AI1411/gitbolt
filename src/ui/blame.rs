//! Inline / Smart Blame helpers (issue #22).
//!
//! Diff-line chips live in [`crate::ui::diff`]; this module re-exports formatters
//! for Context / File Blame surfaces.

pub use crate::app::blame_format::{format_hover, format_minimal};

/// Placeholder retained for module stability.
pub struct BlameView;
