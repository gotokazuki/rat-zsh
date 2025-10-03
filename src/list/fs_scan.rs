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

/// Find candidate directories for `fpath` lookup associated with a plugin slug.
///
/// This function searches under the `plugins_dir` for an entry (directory or
/// symlink) whose slug (as derived from `extract_slug_from`) matches the given
/// `slug`. It then inspects that plugin directory to determine which subdirectories
/// should be added to the Zsh `fpath`.
///
/// A directory is considered a valid `fpath` candidate if:
/// - It contains at least one file starting with an underscore (`_`) — the Zsh
///   convention for completion function files.
/// - Or it is a subdirectory of the plugin root that also meets the same condition.
/// - Directories known to be irrelevant (e.g., `docs`, `tests`, `node_modules`)
///   are skipped.
///
/// ### Arguments
/// - `plugins_dir`: Root directory containing plugin entries (usually `~/.rz/plugins`).
/// - `slug`: The canonical identifier for the plugin (e.g., `"zsh-users__zsh-completions"`).
///
/// ### Returns
/// - `Ok(Vec<String>)`: A list of directories (absolute paths) that should be
///   appended to `fpath`, sorted alphabetically.
/// - `Ok([])`: If no matching slug is found or no valid completion directories exist.
/// - `Err`: If an I/O error occurs while traversing directories (other than `NotFound`).
///
/// ### Notes
/// - Only the first matching plugin entry is considered; if multiple exist,
///   the first is returned.
/// - Hidden directories (names starting with `.`) and well-known non-source
///   directories (see `BLOCK` list) are ignored.
/// - Symbolic links are resolved and canonicalized if possible.
pub fn fpath_dirs_for_slug(plugins_dir: &Path, slug: &str) -> std::io::Result<Vec<String>> {
    use std::fs;

    const BLOCK: &[&str] = &[
        "docs",
        "doc",
        "examples",
        "example",
        "samples",
        "sample",
        "tests",
        "test",
        "spec",
        "scripts",
        "script",
        "tools",
        "bin",
        "assets",
        "images",
        "img",
        "node_modules",
    ];

    fn looks_like_completion_dir(dir: &Path) -> bool {
        if let Ok(rd) = std::fs::read_dir(dir) {
            for ent in rd.flatten() {
                let ft = match ent.file_type() {
                    Ok(t) => t,
                    Err(_) => continue,
                };
                if ft.is_file()
                    && let Some(name) = ent.file_name().to_str()
                    && name.starts_with('_')
                {
                    return true;
                }
            }
        }
        false
    }

    let rd = match fs::read_dir(plugins_dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(e),
    };

    for ent in rd {
        let ent = ent?;
        let ft = ent.file_type()?;
        if !(ft.is_symlink() || ft.is_dir()) {
            continue;
        }
        let path = ent.path();

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

        if extract_slug_from(&target).as_deref() != Some(slug) {
            continue;
        }

        let mut dirs = Vec::new();

        if looks_like_completion_dir(&target) {
            dirs.push(target.clone());
        }

        if let Ok(sub) = fs::read_dir(&target) {
            for s in sub.flatten() {
                let name_os = s.file_name();
                let name = match name_os.to_str() {
                    Some(n) => n,
                    None => continue,
                };
                if name.starts_with('.') || BLOCK.contains(&name) {
                    continue;
                }
                if s.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    let cand = s.path();
                    if looks_like_completion_dir(&cand) {
                        dirs.push(cand);
                    }
                }
            }
        }

        let mut out: Vec<String> = dirs
            .into_iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        out.sort();
        return Ok(out);
    }

    Ok(vec![])
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
        let got = extract_slug(&p);
        assert_eq!(got.as_deref(), Some("owner__repo"));
    }

    #[test]
    fn extract_slug_none_when_no_repos_component() {
        let p = Path::new("/some/where/owner__repo/file.zsh");
        assert!(extract_slug(p).is_none());
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

#[test]
fn fpath_dirs_respects_block_and_detects_completion_dirs() {
    use std::os::unix::fs as unix_fs;
    let td = tempfile::tempdir().unwrap();
    let plugins = td.path();

    let slug = "z__comp";
    let repo_root = td.path().join("..").join("repos").join(slug);
    std::fs::create_dir_all(&repo_root).unwrap();

    std::fs::write(repo_root.join("_rootcomp"), "").unwrap();

    let d1 = repo_root.join("src");
    std::fs::create_dir_all(&d1).unwrap();
    std::fs::write(d1.join("_bar"), "").unwrap();

    let d2 = repo_root.join("docs");
    std::fs::create_dir_all(&d2).unwrap();
    std::fs::write(d2.join("_ignored"), "").unwrap();

    std::fs::create_dir_all(plugins).unwrap();
    unix_fs::symlink(&repo_root, plugins.join("link-to-repo")).unwrap();

    let mut got = fpath_dirs_for_slug(plugins, slug).unwrap();
    got.sort();

    let repo_root_c = std::fs::canonicalize(&repo_root).unwrap();
    let d1_c = std::fs::canonicalize(&d1).unwrap();
    let d2_c = std::fs::canonicalize(&d2).unwrap();

    assert!(got.iter().any(|p| Path::new(p) == repo_root_c));
    assert!(got.iter().any(|p| Path::new(p) == d1_c));
    assert!(!got.iter().any(|p| Path::new(p) == d2_c));
}

#[test]
fn fpath_dirs_returns_empty_for_unknown_slug() {
    let td = tempfile::tempdir().unwrap();
    let got = fpath_dirs_for_slug(td.path(), "no_such").unwrap();
    assert!(got.is_empty());
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
