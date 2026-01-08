//! Crate entry point for **rat-zsh (rz)**.
//!
//! This library provides the internal implementation for the `rz` CLI.
//! Each submodule encapsulates one responsibility (config parsing, git operations, sync logic, etc.).
//! The `pub use` re-exports make selected commands accessible directly from the crate root.
//!
//! This file is primarily intended for developers hacking on `rz`.

mod settings;
mod git;
mod init;
mod list;
mod paths;
mod progress;
mod sync;
mod upgrade;

/// Re-export commonly used types and commands so they can be accessed from `rz::*`.
pub use settings::{Config, Plugin};
pub use init::cmd_init;
pub use list::cmd_list;
pub use paths::rz_home;
pub use sync::cmd_sync;
pub use upgrade::cmd_upgrade;
