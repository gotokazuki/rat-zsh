use anyhow::Result;
use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path};

use super::PluginEntry;

pub fn collect_plugins(
    dir: &Path,
    tail_slugs: &[String],
) -> Result<(Vec<PluginEntry>, Vec<PluginEntry>)> {
    let mut normal = Vec::new();
    let mut tail = Vec::new();

    let rd = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok((normal, tail)),
        Err(e) => return Err(e.into()),
    };

    for ent in rd {
        let ent = match ent {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = ent.path();

        let ft = match ent.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if !(ft.is_file() || ft.is_symlink() || ft.is_dir()) {
            continue;
        }

        let target = if ft.is_symlink() {
            match fs::read_link(&path) {
                Ok(link) => {
                    let abs = if link.is_absolute() {
                        link
                    } else {
                        path.parent().unwrap_or(Path::new(".")).join(link)
                    };
                    fs::canonicalize(&abs).unwrap_or(abs)
                }
                Err(_) => path.clone(),
            }
        } else {
            path.clone()
        };

        if let Some(slug) = extract_slug(&target) {
            let display = slug.replace("__", "/");
            let item = PluginEntry {
                slug: slug.clone(),
                display,
            };
            if tail_slugs.iter().any(|t| t == &slug) {
                tail.push(item);
            } else {
                normal.push(item);
            }
        } else {
            let disp = path
                .file_name()
                .and_then(OsStr::to_str)
                .unwrap_or_default()
                .to_string();
            normal.push(PluginEntry {
                slug: String::new(),
                display: disp,
            });
        }
    }

    Ok((normal, tail))
}

fn extract_slug(target: &Path) -> Option<String> {
    let mut comps = target.components().peekable();
    while let Some(c) = comps.next() {
        if matches!(c, Component::Normal(x) if x == OsStr::new("repos"))
            && let Some(Component::Normal(next)) = comps.next()
        {
            return Some(next.to_string_lossy().into_owned());
        }
    }
    None
}
