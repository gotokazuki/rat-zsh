use anyhow::Result;

use crate::paths::paths;

use super::fs_scan;

/// Represents a discovered plugin entry.
///
/// - `slug`: Internal identifier derived from the repo path, e.g.
///   `"zsh-users__zsh-autosuggestions"`.
/// - `display`: Human-friendly display name, e.g.
///   `"zsh-users/zsh-autosuggestions"`.
#[derive(Debug, Clone)]
pub struct PluginEntry {
    pub slug: String,
    pub display: String,
}

/// Resolve the effective plugin load order.
///
/// This function scans the `plugins/` directory and classifies plugins into
/// two groups:
/// - **normal**: all plugins not explicitly marked as "tail"
/// - **tail**: plugins that must be loaded last (e.g., `zsh-autosuggestions`,
///   `zsh-syntax-highlighting`)
///
/// The returned order is:
/// 1. All normal plugins, sorted alphabetically by display name
/// 2. All tail plugins, in the fixed order specified by `TAIL_SLUGS`
///
/// # Returns
/// - `Ok(Vec<PluginEntry>)`: ordered list of plugins
/// - `Err`: if scanning the plugin directory fails
pub fn resolve_order() -> Result<Vec<PluginEntry>> {
    let p = paths()?;

    const TAIL_SLUGS: &[&str] = &[
        "zsh-users__zsh-autosuggestions",
        "zsh-users__zsh-syntax-highlighting",
    ];

    let (mut normal, tail) = fs_scan::collect_plugins(
        &p.plugins,
        &TAIL_SLUGS.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
    )?;

    normal.sort_by(|a, b| a.display.cmp(&b.display));

    let mut ordered = Vec::with_capacity(normal.len() + tail.len());
    ordered.extend(normal);

    for s in TAIL_SLUGS {
        for t in &tail {
            if &t.slug == s {
                ordered.push(t.clone());
            }
        }
    }

    Ok(ordered)
}
