//! Application layer: state, events, commands, and messages.
//!
//! See `docs/design/05-architecture.md` sections 8–9.

pub mod command;
pub mod event;
pub mod executor;
pub mod message;
pub mod model;
pub mod recent;
pub mod reducer;
pub mod session;
pub mod state;
