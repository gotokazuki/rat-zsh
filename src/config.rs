use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;

use crate::paths::{Paths, paths};

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
    #[serde(default)]
    pub fpath_dirs: Vec<String>,
}

/// Load and parse a configuration file from the given path.
///
/// This is a lower-level helper used by [`load_config`], which always
/// resolves to the default path (`~/.rz/config.toml`).
///
/// # Arguments
/// - `path`: Path to a TOML file containing plugin configuration.
///
/// # Returns
/// A [`Config`] struct with parsed plugin definitions.
///
/// # Errors
/// - Returns an error if the file does not exist or cannot be read.
/// - Returns an error if the TOML content is invalid.
pub fn load_config_from(path: &std::path::Path) -> Result<Config> {
    let txt = fs::read_to_string(path)
        .with_context(|| format!("config not found: {}", path.display()))?;
    let cfg: Config = toml::from_str(&txt).context("failed to parse config.toml")?;
    Ok(cfg)
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
    let p: Paths = paths()?;
    load_config_from(&p.config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn load_config_from_reads_valid_toml() {
        let tmp = tempdir().unwrap();
        let cfg_path = tmp.path().join("config.toml");
        let toml = r#"
            [[plugins]]
            source = "github"
            repo   = "zsh-users/zsh-autosuggestions"
            type   = "source"
            file   = "zsh-autosuggestions.zsh"
        "#;
        std::fs::File::create(&cfg_path)
            .unwrap()
            .write_all(toml.as_bytes())
            .unwrap();

        let cfg = load_config_from(&cfg_path).unwrap();
        assert_eq!(cfg.plugins.len(), 1);
        assert_eq!(cfg.plugins[0].repo, "zsh-users/zsh-autosuggestions");
    }

    #[test]
    fn load_config_from_errors_when_missing() {
        let tmp = tempdir().unwrap();
        let cfg_path = tmp.path().join("nope.toml");
        let err = load_config_from(&cfg_path).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("config not found"));
        assert!(msg.contains("nope.toml"));
    }
}
