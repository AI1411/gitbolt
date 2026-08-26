//! Application layer: state, events, commands, and messages.
//!
//! See `docs/design/05-architecture.md` sections 8–9.

pub mod blame_format;
pub mod branch_cleanup;
pub mod branch_health;
pub mod command;
pub mod conventional;
pub mod diff_parse;
pub mod event;
pub mod executor;
pub mod heatmap;
pub mod issue_link;
pub mod message;
pub mod model;
pub mod palette;
pub mod pulse;
pub mod quick_open;
pub mod recent;
pub mod reducer;
pub mod session;
pub mod state;
