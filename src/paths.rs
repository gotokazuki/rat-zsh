use anyhow::Result;
use std::{env, path::PathBuf};

/// Holds important directory paths used by rat-zsh.
///
/// - `bin`: directory where the `rz` binary is placed
/// - `plugins`: directory where plugin files are stored
/// - `repos`: directory where plugin repositories are cloned
/// - `config`: path to the `config.toml` configuration file
#[derive(Clone)]
pub struct Paths {
    pub bin: PathBuf,
    pub plugins: PathBuf,
    pub repos: PathBuf,
    pub config: PathBuf,
}

/// Returns the base directory for rat-zsh (`$XDG_CONFIG_HOME/.rz`).
///
/// If `$XDG_CONFIG_HOME` is not set, falls back to `$HOME/.config/.rz`.
pub fn rz_home() -> Result<PathBuf> {
    let xdg = env::var_os("XDG_CONFIG_HOME");
    let base = xdg
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(".config"));
    Ok(base.join(".rz"))
}

/// Returns a `Paths` struct with the resolved directories used by rat-zsh.
///
/// This includes:
/// - `bin` (`rz_home()/bin`)
/// - `plugins` (`rz_home()/plugins`)
/// - `repos` (`rz_home()/repos`)
/// - `config` (`rz_home()/config.toml`)
pub fn paths() -> Result<Paths> {
    let home = rz_home()?;
    Ok(Paths {
        bin: home.join("bin"),
        plugins: home.join("plugins"),
        repos: home.join("repos"),
        config: home.join("config.toml"),
    })
}
