use anyhow::Result;
use clap::{Parser, Subcommand};
use rz::{cmd_init, cmd_list, cmd_order, cmd_sync, cmd_upgrade, rz_home};

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
    /// Show rz home directory
    Home,
    /// Show the effective plugin load order
    Order,
}

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
        Cmd::Order => cmd_order(),
    }
}
