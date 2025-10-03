use crate::config::load_config;
use crate::paths::paths;
use anyhow::Result;
use order::{PluginEntry, resolve_order};
use std::collections::HashMap;

mod fs_scan;
mod order;

/// CLI command: print the list of plugins in their effective load order.
///
/// This command combines information from both the configuration file
/// (`config.toml`) and the on-disk plugin directory:
///
/// - The **load order** is determined by scanning the `plugins/` directory
///   and applying the ordering rules (normal plugins sorted by display name,
///   followed by "tail" plugins such as `zsh-autosuggestions`).
/// - For each plugin, if configuration metadata is available, the line
///   includes the configured `name`, `source` (e.g., `"github"`), and type
///   (`"source"`, `"fpath"`, etc.).
/// - Plugins without metadata (e.g., plain files or broken symlinks in
///   `plugins/`) are still listed, but without extra fields.
///
/// Example output:
/// ```text
/// - olets/zsh-abbr (github) [source]
/// - zsh-users/zsh-completions (github) [fpath]
/// - zsh-users/zsh-history-substring-search (github) [source]
/// - zsh-users/zsh-autosuggestions (github) [source]
/// - zsh-users/zsh-syntax-highlighting (github) [source]
/// ```
///
/// # Errors
/// Returns an error if the configuration cannot be loaded or if plugin
/// directory scanning fails.
pub fn cmd_list() -> Result<()> {
    let cfg = load_config()?;
    let mut meta: HashMap<String, (String, String, Option<String>)> = HashMap::new();
    for pl in cfg.plugins {
        let slug = pl.repo.replace('/', "__");
        let source = pl.source.clone();
        let ty = pl.r#type.as_deref().unwrap_or("source").to_string();
        let name = pl.name.clone();
        meta.insert(slug, (source, ty, name));
    }

    println!("Source order");
    let ordered: Vec<PluginEntry> = resolve_order()?;

    for e in &ordered {
        if let Some((source, ty, name)) = meta.get(&e.slug).cloned() {
            if ty == "fpath" {
                continue;
            }
            let shown = name.unwrap_or_else(|| e.display.clone());
            println!("- {} ({}) [source]", shown, source);
        } else {
            println!("- {}", e.display);
        }
    }

    let p = paths()?;
    let fpaths = fs_scan::collect_fpath_dirs(&p.plugins)?;

    println!("\nfpath");
    for f in &fpaths {
        if let Some((source, ty, name)) = meta.get(&f.slug).cloned() {
            if ty == "fpath" {
                let shown = name.unwrap_or_else(|| f.display.clone());
                println!("- {} ({}) [fpath: {}]", shown, source, f.dir.display());
            } else {
                let shown = name.unwrap_or_else(|| f.display.clone());
                println!("- {} ({}) [dir: {}]", shown, source, f.dir.display());
            }
        } else {
            println!("- {} [fpath: {}]", f.display, f.dir.display());
        }
    }

    Ok(())
}
