use anyhow::Result;
use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path};

use super::PluginEntry;

/// Collect plugins from the given directory and classify them into
/// "normal" and "tail" groups.
///
/// - A plugin is identified by a "slug", usually derived from the path
///   segment after `repos/`.
/// - Entries whose slug matches `tail_slugs` will be placed in the `tail`
///   group (these are typically plugins that must be loaded after others).
/// - All other plugins are placed in the `normal` group.
/// - Files, symlinks, and directories are all considered.
/// - Broken symlinks or unreadable entries are skipped.
///
/// # Arguments
/// - `dir`: Directory containing plugin files/symlinks.
/// - `tail_slugs`: List of slugs that should be loaded last.
///
/// # Returns
/// A tuple `(normal, tail)` where both are `Vec<PluginEntry>`.
///
/// # Errors
/// Returns an error if the directory cannot be read for reasons other than
/// `NotFound`. If the directory does not exist, returns empty vectors.
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

/// Try to extract a plugin "slug" from the given path.
///
/// A slug is defined as the component immediately following a `repos`
/// directory in the path (e.g., `.../repos/<slug>/...`).
///
/// Example:
/// - `/home/user/.rz/repos/zsh-users__zsh-autosuggestions/...`
///   → returns `"zsh-users__zsh-autosuggestions"`.
///
/// # Arguments
/// - `target`: Path to examine.
///
/// # Returns
/// `Some(slug)` if found, otherwise `None`.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs as unix_fs;
    use tempfile::tempdir;

    fn displays(mut v: Vec<super::PluginEntry>) -> Vec<String> {
        v.sort_by(|a, b| a.display.cmp(&b.display));
        v.into_iter().map(|e| e.display).collect()
    }

    #[test]
    fn extract_slug_finds_component_after_repos() {
        let base = Path::new("/home/user/.rz");
        let p = base
            .join("repos")
            .join("owner__repo")
            .join("path")
            .join("file.zsh");
        let got = super::extract_slug(&p);
        assert_eq!(got.as_deref(), Some("owner__repo"));
    }

    #[test]
    fn extract_slug_none_when_no_repos_component() {
        let p = Path::new("/some/where/owner__repo/file.zsh");
        assert!(super::extract_slug(p).is_none());
    }

    #[test]
    fn collect_plugins_classifies_normal_and_tail() {
        let td = tempdir().unwrap();
        let base = td.path();

        let repos = base.join("repos");
        let slug_tail = "zsh-users__zsh-syntax-highlighting";
        let slug_norm = "zsh-users__zsh-autosuggestions";
        fs::create_dir_all(repos.join(slug_tail)).unwrap();
        fs::create_dir_all(repos.join(slug_norm)).unwrap();

        let file_tail = repos.join(slug_tail).join("zsh-syntax-highlighting.zsh");
        let file_norm = repos.join(slug_norm).join("zsh-autosuggestions.zsh");
        fs::write(&file_tail, "# tail plugin").unwrap();
        fs::write(&file_norm, "# normal plugin").unwrap();

        let plugins = base.join("plugins");
        fs::create_dir_all(&plugins).unwrap();
        let link_tail = plugins.join("tail-link");
        let link_norm = plugins.join("norm-link");
        unix_fs::symlink(&file_tail, &link_tail).unwrap();
        unix_fs::symlink(&file_norm, &link_norm).unwrap();

        let plain = plugins.join("plain.plugin.zsh");
        fs::write(&plain, "# plain").unwrap();

        let broken = plugins.join("broken-link");
        unix_fs::symlink(plugins.join("no/such/file.zsh"), &broken).unwrap();

        let tail_slugs = vec![slug_tail.to_string()];

        let (normal, tail) = collect_plugins(&plugins, &tail_slugs).unwrap();

        let tail_disp = displays(tail);
        assert_eq!(tail_disp, vec!["zsh-users/zsh-syntax-highlighting"]);

        let mut norm_disp = displays(normal);
        norm_disp.sort();
        let mut expected = vec![
            "plain.plugin.zsh".to_string(),
            "broken-link".to_string(),
            "zsh-users/zsh-autosuggestions".to_string(),
        ];
        expected.sort();
        assert_eq!(norm_disp, expected);
    }

    #[test]
    fn collect_plugins_returns_empty_when_dir_not_found() {
        let td = tempdir().unwrap();
        let missing = td.path().join("no_such_dir");
        let (normal, tail) = collect_plugins(&missing, &[]).unwrap();
        assert!(normal.is_empty());
        assert!(tail.is_empty());
    }
}
