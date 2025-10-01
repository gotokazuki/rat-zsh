//! # rat-zsh (rz)
//!
//! **rz** is a minimal Zsh plugin manager.
//!
//! Features:
//! - Manage plugins defined in `$(rz home)/.rz/config.toml`
//! - `rz init` prints initialization code for `.zshrc`
//! - `rz sync` clones or updates configured plugins
//! - `rz upgrade` updates rz itself to the latest release
//! - `rz list` show plugins in the effective load order with source/type metadata
//! - `rz home` prints the rz home directory
//!
//! This CLI is built with [clap](https://docs.rs/clap).

use anyhow::Result;
use clap::{Parser, Subcommand};
use rz::{cmd_init, cmd_list, cmd_sync, cmd_upgrade, rz_home};

/// Command-line interface definition.
///
/// Parsed using `clap` derive macros.
#[derive(Parser, Debug)]
#[command(
    name = "rz",
    version,
    about = "rat-zsh (rz) - minimal zsh plugin manager",
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

/// Available subcommands.
///
/// Each variant corresponds to a subcommand of `rz`.
#[derive(Subcommand, Debug)]
enum Cmd {
    /// Print initialization code for .zshrc
    Init,
    /// Clone/update plugins defined in config.toml
    Sync,
    /// Update rat-zsh itself to the latest release
    Upgrade,
    /// List parsed plugins
    List,
    /// Show plugins in the effective load order with source/type metadata
    Home,
}

/// CLI entry point.
///
/// Parses arguments with `clap` and executes the selected subcommand.
fn main() -> Result<()> {
    let cli = Cli::parse();
    let cmd = cli.cmd.unwrap();

    match cmd {
        Cmd::Init => cmd_init(),
        Cmd::Sync => cmd_sync(),
        Cmd::Upgrade => cmd_upgrade(),
        Cmd::List => cmd_list(),
        Cmd::Home => {
            println!("{}", rz_home()?.display());
            Ok(())
        }
    }
}
