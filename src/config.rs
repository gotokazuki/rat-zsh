use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;

use crate::paths::paths;

/// Top-level configuration structure loaded from `config.toml`.
///
/// The file defines how plugins are managed by rat-zsh.
/// Currently, only the `plugins` section is supported.
///
/// Example TOML:
/// ```toml
/// [[plugins]]
/// source = "github"
/// repo   = "zsh-users/zsh-autosuggestions"
/// type   = "source"
/// file   = "zsh-autosuggestions.zsh"
/// ```
#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub plugins: Vec<Plugin>,
}

/// Representation of a single plugin entry in `config.toml`.
///
/// Each field corresponds to keys typically found under `[[plugins]]`.
/// All fields are optional (default empty or `None`) to allow flexible configs.
#[derive(Debug, Deserialize, Clone)]
pub struct Plugin {
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub repo: String,
    #[serde(default)]
    pub rev: Option<String>,
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

/// Load and parse `config.toml` into a [`Config`] structure.
///
/// # Errors
/// - Returns an error if `config.toml` cannot be read.
/// - Returns an error if parsing the TOML fails.
///
/// # Notes
/// - This always resolves the path using [`paths()`].
/// - If the file is missing, the error message includes the resolved path.
pub fn load_config() -> Result<Config> {
    let p = paths()?;
    let txt = fs::read_to_string(&p.config)
        .with_context(|| format!("config not found: {}", p.config.display()))?;
    let cfg: Config = toml::from_str(&txt).context("failed to parse config.toml")?;
    Ok(cfg)
}

/// CLI command: print a human-readable list of plugins.
///
/// Each plugin is displayed with:
/// - name or repo (for identification)
/// - source (e.g., `github`)
/// - type (`source`, `fpath`, etc.)
///
/// Example output:
/// ```text
/// - zsh-autosuggestions (github) [source]
/// - zsh-completions (github) [fpath]
/// ```
///
/// # Errors
/// - Returns an error if `config.toml` cannot be loaded or parsed.
pub fn cmd_list() -> Result<()> {
    let cfg = load_config()?;
    for pl in cfg.plugins {
        let t = pl.r#type.as_deref().unwrap_or("source");
        let display = pl.name.as_deref().unwrap_or(&pl.repo);
        println!("- {} ({}) [{}]", display, pl.source, t);
    }
    Ok(())
}
