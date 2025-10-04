use crate::config::load_config;
use crate::paths::paths;
use anyhow::Result;
use order::{PluginEntry, resolve_order};
use std::collections::HashMap;

mod fs_scan;
mod order;

/// Print plugins in their effective load order, split by **source** and **fpath** roles.
///
/// This command merges information from:
/// - The on-disk `plugins/` directory (actual load order and presence)
/// - `config.toml` (metadata: `name`, `source`, `type`)
///
/// ### Output format
/// The output is split into two sections:
///
/// 1. **`Source order`**
///    - Shows plugins whose `type` is **not** `fpath`.
///    - Order is computed from the `plugins/` directory:
///      - “Normal” plugins sorted alphabetically by display name
///      - “Tail” plugins (e.g. `zsh-autosuggestions`, `zsh-syntax-highlighting`) appended last in a fixed order
///    - Each line includes the display name (or configured `name`), the `source` (e.g. `github`), and the literal tag `[source]`.
///
/// 2. **`fpath`**
///    - Shows plugins whose `type` **is** `fpath`.
///    - For each plugin, candidate completion directories under its repository are discovered
///      (ignoring dot-directories like `.git`, `.github`, etc. and obvious non-completion folders).
///    - If one directory is found, it is shown after `fpath:` as an absolute path.
///      If multiple are found, they are shown as `{dir1, dir2, ...}` to indicate search order.
///      If none are found, only `[fpath]` is printed without a path.
///
/// ### Example
/// ```text
/// Source order
/// - olets/zsh-abbr (github) [source]
/// - zsh-users/zsh-history-substring-search (github) [source]
/// - zsh-users/zsh-autosuggestions (github) [source]
/// - zsh-users/zsh-syntax-highlighting (github) [source]
///
/// fpath
/// - zsh-users/zsh-completions (github) [fpath: /home/user/.rz/plugins/zsh-users__zsh-completions/src]
/// ```
///
/// Notes:
/// - Entries lacking config metadata are still listed but without extra fields.
/// - “Tail” membership is determined by a fixed list and ensures those plugins load last.
///
/// # Errors
/// Returns an error if configuration loading fails or if plugin directory scanning fails.
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

    println!("\nfpath");
    for e in &ordered {
        if e.slug.is_empty() {
            continue;
        }
        if let Some((source, ty, name)) = meta.get(&e.slug).cloned() {
            if ty != "fpath" {
                continue;
            }
            let shown = name.unwrap_or_else(|| e.display.clone());
            let dirs = fs_scan::fpath_dirs_for_slug(&p.plugins, &e.slug).unwrap_or_default();
            let suffix = fs_scan::format_fpath_dirs(&dirs);
            if suffix.is_empty() {
                println!("- {} ({}) [fpath]", shown, source);
            } else {
                println!("- {} ({}) [fpath: {}]", shown, source, suffix);
            }
        }
    }

    Ok(())
}
