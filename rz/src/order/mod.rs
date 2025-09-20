mod fs_scan;

use crate::paths::paths;
use anyhow::Result;

#[derive(Debug, Clone)]
struct PluginEntry {
    slug: String,
    display: String,
}

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
