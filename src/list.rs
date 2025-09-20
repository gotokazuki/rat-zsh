use crate::config::load_config;
use anyhow::Result;

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
