mod fs_scan;

use crate::paths::paths;
use anyhow::Result;

/// Represents a discovered plugin entry.
/// - `slug`: Internal identifier, usually derived from the repository name
///   (e.g., `"zsh-users__zsh-autosuggestions"`).
/// - `display`: Human-friendly string for printing (e.g., `"zsh-users/zsh-autosuggestions"`).
#[derive(Debug, Clone)]
struct PluginEntry {
    slug: String,
    display: String,
}

/// Print the effective plugin load order.
///
/// This command is invoked by `rz order`.
/// It scans the plugin directory and lists plugins in the order
/// they will be sourced in `.zshrc`.
///
/// - "normal" plugins are listed first, sorted alphabetically.
/// - Some plugins (like `zsh-autosuggestions` and
///   `zsh-syntax-highlighting`) must be loaded at the very end,
///   so they are placed into the `tail` group and printed last.
///
/// # Returns
/// - `Ok(())` on success.
/// - `Err` if scanning the plugin directory fails (other than "not found").
pub fn cmd_order() -> Result<()> {
    let p = paths()?;
    let tail_slugs = vec![
        "zsh-users__zsh-autosuggestions".to_string(),
        "zsh-users__zsh-syntax-highlighting".to_string(),
    ];

    let (mut normal, tail) = fs_scan::collect_plugins(&p.plugins, &tail_slugs)?;
    normal.sort_by(|a, b| a.display.cmp(&b.display));

    for n in &normal {
        println!("- {}", n.display);
    }
    for s in &tail_slugs {
        for t in &tail {
            if &t.slug == s {
                println!("- {}", t.display);
            }
        }
    }
    Ok(())
}
