//! Git service layer — all Git operations are routed through here.
//!
//! See `docs/design/02-tech-and-performance.md` and `docs/design/05-architecture.md`.

pub mod blame;
pub mod branch;
pub mod commit;
pub mod diff;
pub mod history;
pub mod remote;
pub mod repository;
pub mod status;
pub mod worktree;
