use anyhow::Result;
use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path};

use super::order::PluginEntry;

/// Extract the plugin slug from a given path.
///
/// A *slug* is defined as the path component immediately following a
/// `repos/` directory. This is used as the canonical identifier for a plugin,
/// e.g. `"zsh-users__zsh-autosuggestions"`.
///
/// # Arguments
/// - `path`: Path to examine.
///
/// # Returns
/// - `Some(slug)` if a `repos/` component is found and followed by a normal
///   directory name
/// - `None` if no such component exists
///
/// # Notes
/// - The slug is returned as a `String` with any non-UTF-8 sequences lossy-converted.
/// - Only the **first occurrence** of `repos/` in the path is considered.
pub fn extract_slug_from(path: &Path) -> Option<String> {
    let mut comps = path.components().peekable();
    while let Some(c) = comps.next() {
        if matches!(c, Component::Normal(x) if x == OsStr::new("repos"))
            && let Some(Component::Normal(next)) = comps.next()
        {
            return Some(next.to_string_lossy().into_owned());
        }
    }
    None
}

/// Resolve the list of directories to include in `fpath`, based on explicit
/// configuration entries in `config.toml`.
///
/// This function does **not** perform automatic directory scanning.  
/// Instead, it uses the paths specified under `fpath_dirs` in each plugin’s
/// configuration, resolving them relative to the plugin’s repository root
/// (under `~/.rz/repos/<slug>`).
///
/// # Arguments
///
/// * `plugins_dir` - The root directory containing plugin symlinks (usually `~/.rz/plugins`).
/// * `slug` - The canonical plugin identifier (e.g. `"zsh-users__zsh-completions"`).
/// * `cfg_dirs` - The list of directory paths defined in `config.toml` under `fpath_dirs`.
///
/// # Behavior
///
/// - The function first resolves the **plugin root directory** by finding a symlink or folder
///   under `plugins_dir` that points to the repository corresponding to the given `slug`.
/// - Each entry in `cfg_dirs` is interpreted as:
///   - An **absolute path** (if it starts with `/`), used as-is.
///   - A **relative path**, resolved against the plugin root.
/// - Only existing directories are included in the final output; non-existent entries are ignored.
/// - Paths are canonicalized (via `std::fs::canonicalize`) when possible.
/// - The returned paths are **relative to the plugin root** for readability, e.g.:
///   ```text
///   contrib/completions/zsh
///   src
///   ```
///
/// # Returns
///
/// - `Ok(Vec<String>)`: A sorted, deduplicated list of relative directory paths to include in `fpath`.
/// - `Ok([])`: If no matching plugin is found or none of the configured directories exist.
/// - `Err`: If an I/O error occurs while reading directories (except for `NotFound`).
///
/// # Notes
///
/// - This function intentionally skips any automatic completion detection
///   logic (e.g., scanning for `_foo` files).
/// - It is meant to provide **predictable and explicit fpath resolution**
///   consistent with user-defined configuration.
pub fn fpath_dirs_from_config(
    plugins_dir: &Path,
    slug: &str,
    cfg_dirs: &[String],
) -> std::io::Result<Vec<String>> {
    let mut plugin_root: Option<std::path::PathBuf> = None;
    for ent in std::fs::read_dir(plugins_dir)? {
        let ent = match ent {
            Ok(x) => x,
            Err(_) => continue,
        };
        let ft = match ent.file_type() {
            Ok(x) => x,
            Err(_) => continue,
        };
        if !(ft.is_symlink() || ft.is_dir()) {
            continue;
        }
        let p = ent.path();
        let target = if ft.is_symlink() {
            match std::fs::read_link(&p) {
                Ok(link) => {
                    let abs = if link.is_absolute() {
                        link
                    } else {
                        p.parent().unwrap_or(Path::new(".")).join(link)
                    };
                    std::fs::canonicalize(&abs).unwrap_or(abs)
                }
                Err(_) => p.clone(),
            }
        } else {
            p.clone()
        };
        if super::fs_scan::extract_slug_from(&target).as_deref() == Some(slug) {
            plugin_root = Some(target);
            break;
        }
    }
    let root = match plugin_root {
        Some(r) => r,
        None => return Ok(vec![]),
    };

    let mut out = Vec::new();
    for d in cfg_dirs {
        let cand = {
            let pd = Path::new(d);
            if pd.is_absolute() {
                pd.to_path_buf()
            } else {
                root.join(pd)
            }
        };
        if cand.is_dir() {
            let canon = std::fs::canonicalize(&cand).unwrap_or(cand);
            let display = canon
                .strip_prefix(&root)
                .unwrap_or(&canon)
                .to_string_lossy()
                .into_owned();
            out.push(display);
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

/// Format a list of `fpath` directories into a display-friendly string.
///
/// This helper is used to produce human-readable output for plugins that
/// contribute one or more directories to Zsh's `fpath`.
///
/// ### Behavior
/// - If the list is empty → returns an empty string (`""`).
/// - If the list has exactly one entry → returns that path as-is.
/// - If the list has multiple entries → joins them with `", "` and wraps them
///   in curly braces, e.g. `{dir1, dir2, dir3}`.
///
/// ### Arguments
/// - `dirs`: Slice of directory paths (as `String`s).
///
/// ### Returns
/// - `String` representing the directories in a compact, display-oriented format.
pub fn format_fpath_dirs(dirs: &[String]) -> String {
    match dirs.len() {
        0 => String::new(),
        1 => dirs[0].clone(),
        _ => format!("{{{}}}", dirs.join(", ")),
    }
}

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

        if let Some(slug) = extract_slug_from(&target) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs as unix_fs;
    use tempfile::tempdir;

    fn displays(mut v: Vec<crate::list::order::PluginEntry>) -> Vec<String> {
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
        let got = extract_slug_from(&p);
        assert_eq!(got.as_deref(), Some("owner__repo"));
    }

    #[test]
    fn extract_slug_none_when_no_repos_component() {
        let p = Path::new("/some/where/owner__repo/file.zsh");
        assert!(extract_slug_from(p).is_none());
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

        #[cfg(unix)]
        {
            let link_tail = plugins.join("tail-link");
            let link_norm = plugins.join("norm-link");
            unix_fs::symlink(&file_tail, &link_tail).unwrap();
            unix_fs::symlink(&file_norm, &link_norm).unwrap();
        }

        let plain = plugins.join("plain.plugin.zsh");
        fs::write(&plain, "# plain").unwrap();

        #[cfg(unix)]
        {
            let broken = plugins.join("broken-link");
            unix_fs::symlink(plugins.join("no/such/file.zsh"), &broken).unwrap();
        }

        let tail_slugs = vec![slug_tail.to_string()];

        let (normal, tail) = collect_plugins(&plugins, &tail_slugs).unwrap();

        let tail_disp = displays(tail);
        assert_eq!(tail_disp, vec!["zsh-users/zsh-syntax-highlighting"]);

        let mut norm_disp = displays(normal);
        norm_disp.sort();
        let mut expected = vec![
            "plain.plugin.zsh".to_string(),
            #[cfg(unix)]
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

    #[test]
    fn fpath_from_config_returns_only_existing_configured_dirs() {
        use std::os::unix::fs as unix_fs;

        let td = tempfile::tempdir().unwrap();
        let plugins = td.path().join("plugins");
        let repos = td.path().join("repos");
        fs::create_dir_all(&plugins).unwrap();

        let slug = "owner__repo";
        let repo_root = repos.join(slug);
        fs::create_dir_all(&repo_root).unwrap();

        let d1 = repo_root.join("src");
        let d2 = repo_root.join("contrib").join("completions").join("zsh");
        fs::create_dir_all(&d1).unwrap();
        fs::create_dir_all(d2.parent().unwrap()).unwrap();
        fs::create_dir_all(&d2).unwrap();

        unix_fs::symlink(&repo_root, plugins.join("link-to-repo")).unwrap();

        let cfg_dirs = vec![
            ".".to_string(),
            "src".to_string(),
            "contrib/completions/zsh".to_string(),
            "nope".to_string(),
        ];

        let mut got = fpath_dirs_from_config(&plugins, slug, &cfg_dirs).unwrap();
        got.sort();

        let expect = vec![
            "".to_string(),
            "contrib/completions/zsh".to_string(),
            "src".to_string(),
        ];

        assert_eq!(got, expect);
    }

    #[test]
    fn fpath_from_config_returns_empty_for_unknown_slug() {
        let td = tempfile::tempdir().unwrap();
        let got = fpath_dirs_from_config(td.path(), "no_such", &["src".into()]).unwrap();
        assert!(got.is_empty());
    }

    #[test]
    fn extract_slug_none_when_repos_is_last_segment() {
        let p = Path::new("/home/u/.rz/repos");
        assert!(extract_slug_from(p).is_none());
    }

    #[test]
    fn extract_slug_uses_first_repos_occurrence() {
        let p = Path::new("/a/repos/first/x/repos/second/y");
        assert_eq!(extract_slug_from(p).as_deref(), Some("first"));
    }

    #[cfg(unix)]
    #[test]
    fn extract_slug_handles_non_utf8() {
        use std::os::unix::ffi::OsStrExt;
        let mut path = std::path::PathBuf::from("/home/u/.rz/repos");
        path.push(std::ffi::OsStr::from_bytes(b"zsh-users__\xFF\xFE"));
        path.push("file.zsh");
        assert!(extract_slug_from(&path).is_some());
    }

    #[test]
    fn format_fpath_dirs_formats_variants() {
        assert_eq!(format_fpath_dirs(&[]), "");
        assert_eq!(format_fpath_dirs(&[String::from("/a")]), "/a");
        assert_eq!(
            format_fpath_dirs(&[String::from("/a"), String::from("/b")]),
            "{/a, /b}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn collect_plugins_classifies_and_skips_broken_symlink() {
        use std::os::unix::fs as unix_fs;

        let td = tempfile::tempdir().unwrap();
        let base = td.path();
        let plugins = base.join("plugins");
        let repos = base.join("repos");
        std::fs::create_dir_all(&plugins).unwrap();
        std::fs::create_dir_all(&repos).unwrap();

        let slug_norm = "owner__norm";
        let slug_tail = "owner__tail";
        let f_norm = repos.join(slug_norm).join("a.zsh");
        let f_tail = repos.join(slug_tail).join("b.zsh");
        std::fs::create_dir_all(f_norm.parent().unwrap()).unwrap();
        std::fs::create_dir_all(f_tail.parent().unwrap()).unwrap();
        std::fs::write(&f_norm, "").unwrap();
        std::fs::write(&f_tail, "").unwrap();

        unix_fs::symlink(&f_norm, plugins.join("norm")).unwrap();
        unix_fs::symlink(&f_tail, plugins.join("tail")).unwrap();

        unix_fs::symlink(plugins.join("nope"), plugins.join("broken")).unwrap();

        let tails = vec![slug_tail.to_string()];
        let (normal, tail) = collect_plugins(&plugins, &tails).unwrap();

        let norm_names: Vec<_> = normal.into_iter().map(|e| e.display).collect();
        assert!(norm_names.contains(&"owner/norm".to_string()));

        let tail_names: Vec<_> = tail.into_iter().map(|e| e.display).collect();
        assert_eq!(tail_names, vec!["owner/tail".to_string()]);
    }
}
