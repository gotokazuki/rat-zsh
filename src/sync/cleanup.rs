use anyhow::Result;
use indicatif::{MultiProgress, ProgressBar};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path};
use std::time::Duration;

use super::progress::{err_style, ok_style, spinner_style};

/// Remove stale plugin entries from the plugin directory.
///
/// A "plugin entry" here is usually a symlink inside `~/.rz/plugins`
/// pointing to the actual file in the `repos` tree. If an entry is found
/// in the filesystem but is **not present in `expect`**, it will be deleted.
///
/// A spinner progress bar is shown for each removal.
///
/// # Arguments
/// - `mp`: `MultiProgress` instance for rendering multiple progress bars.
/// - `plugins_dir`: Path to the plugins directory (`~/.rz/plugins`).
/// - `expect`: Set of plugin names that should exist (all others are considered stale).
///
/// # Errors
/// Returns `Err` if filesystem operations fail (other than "not found").
pub fn cleanup_stale_plugins(
    mp: &MultiProgress,
    plugins_dir: &Path,
    expect: &HashSet<String>,
) -> Result<()> {
    let rd = match fs::read_dir(plugins_dir) {
        Ok(rd) => rd,
        Err(_) => return Ok(()),
    };

    for ent in rd.flatten() {
        let name = ent.file_name().to_string_lossy().to_string();
        if expect.contains(&name) {
            continue;
        }

        let pb = mp.add(ProgressBar::new_spinner());
        pb.set_style(spinner_style());
        pb.set_message(format!("removing stale plugin: {}", name));
        pb.enable_steady_tick(Duration::from_millis(80));

        match fs::remove_file(ent.path()) {
            Ok(_) => {
                pb.set_style(ok_style());
                pb.finish_with_message(format!("removed plugin: {}", name));
            }
            Err(e) => {
                pb.set_style(err_style());
                pb.finish_with_message(format!("remove plugin {} (error: {})", name, e));
            }
        }
    }
    Ok(())
}

/// Remove stale repositories from the repos directory.
///
/// A "repo" here corresponds to a cloned Git repository inside
/// `~/.rz/repos/<slug>`. A repo is considered stale if:
/// - It is not explicitly expected (`expect_slugs`), **and**
/// - No plugin symlink in `plugins_dir` points into it.
///
/// If such a repo is found, it will be recursively deleted.
///
/// # Arguments
/// - `mp`: `MultiProgress` instance for rendering multiple progress bars.
/// - `repos_dir`: Path to the repos directory (`~/.rz/repos`).
/// - `expect_slugs`: Set of repository slugs that should exist.
/// - `plugins_dir`: Path to the plugins directory (used to detect symlink targets).
///
/// # Errors
/// Returns `Err` if filesystem operations fail (other than "not found").
pub fn cleanup_stale_repos(
    mp: &MultiProgress,
    repos_dir: &Path,
    expect_slugs: &HashSet<String>,
    plugins_dir: &Path,
) -> Result<()> {
    let mut in_use: HashSet<String> = HashSet::new();
    if let Ok(rd) = fs::read_dir(plugins_dir) {
        for ent in rd.flatten() {
            if let Ok(target) = fs::read_link(ent.path())
                && let Some(slug) = extract_slug(&fs::canonicalize(&target).unwrap_or(target))
            {
                in_use.insert(slug);
            }
        }
    }

    if let Ok(rd) = fs::read_dir(repos_dir) {
        for ent in rd.flatten() {
            let slug = ent.file_name().to_string_lossy().to_string();
            if expect_slugs.contains(&slug) || in_use.contains(&slug) {
                continue;
            }

            let pb = mp.add(ProgressBar::new_spinner());
            pb.set_style(spinner_style());
            pb.set_message(format!("removing stale repo: {}", slug));
            pb.enable_steady_tick(Duration::from_millis(80));

            match fs::remove_dir_all(ent.path()) {
                Ok(_) => {
                    pb.set_style(ok_style());
                    pb.finish_with_message(format!("removed repo: {}", slug));
                }
                Err(e) => {
                    pb.set_style(err_style());
                    pb.finish_with_message(format!("remove repo {} (error: {})", slug, e));
                }
            }
        }
    }
    Ok(())
}

/// Extract the slug (repository identifier) from a plugin symlink target.
///
/// The slug is determined by scanning the path for the `repos/`
/// component and returning the next segment.
///
/// For example:
/// - Input: `~/.rz/repos/zsh-users__zsh-autosuggestions/...`
/// - Output: `"zsh-users__zsh-autosuggestions"`
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
    use indicatif::MultiProgress;
    use std::collections::HashSet;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;

    #[cfg(unix)]
    fn symlink_file(src: &Path, dst: &Path) {
        use std::os::unix::fs::symlink;
        symlink(src, dst).expect("symlink");
    }

    #[cfg(unix)]
    #[test]
    fn cleanup_stale_plugins_removes_unexpected_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let plugins_dir = tmp.path().join("plugins");
        fs::create_dir(&plugins_dir).unwrap();

        let keep = plugins_dir.join("keep.plugin.zsh");
        fs::File::create(&keep).unwrap().write_all(b"ok").unwrap();

        let drop_ = plugins_dir.join("drop.plugin.zsh");
        fs::File::create(&drop_).unwrap().write_all(b"ng").unwrap();

        let mut expect = HashSet::new();
        expect.insert("keep.plugin.zsh".to_string());

        let mp = MultiProgress::new();
        cleanup_stale_plugins(&mp, &plugins_dir, &expect).unwrap();

        assert!(keep.exists());
        assert!(!drop_.exists(), "stale plugin should be removed");
    }

    #[cfg(unix)]
    #[test]
    fn cleanup_stale_repos_removes_unused_and_unexpected_repos() {
        let tmp = tempfile::tempdir().unwrap();
        let repos_dir = tmp.path().join("repos");
        let plugins_dir = tmp.path().join("plugins");
        fs::create_dir(&repos_dir).unwrap();
        fs::create_dir(&plugins_dir).unwrap();

        let usedslug = repos_dir.join("usedslug");
        fs::create_dir(&usedslug).unwrap();
        let used_target = usedslug.join("some.zsh");
        fs::File::create(&used_target)
            .unwrap()
            .write_all(b"echo ok")
            .unwrap();

        let link = plugins_dir.join("some-plugin");
        symlink_file(&used_target, &link);

        let staleslug = repos_dir.join("staleslug");
        fs::create_dir(&staleslug).unwrap();
        fs::File::create(staleslug.join("x")).unwrap();

        let expect_slugs: HashSet<String> = HashSet::new();

        let mp = MultiProgress::new();
        cleanup_stale_repos(&mp, &repos_dir, &expect_slugs, &plugins_dir).unwrap();

        assert!(usedslug.exists(), "in-use repo must be preserved");
        assert!(
            !staleslug.exists(),
            "unused & unexpected repo must be removed"
        );
    }

    #[test]
    fn extract_slug_returns_slug_after_repos_component() {
        let p = PathBuf::from("/home/user/.rz/repos/zsh-users__zsh-autosuggestions/file.zsh");
        let got = super::extract_slug(&p);
        assert_eq!(got.as_deref(), Some("zsh-users__zsh-autosuggestions"));
    }

    #[test]
    fn extract_slug_returns_none_when_no_repos_component() {
        let p = PathBuf::from("/home/user/.rz/plugins/foo.plugin.zsh");
        assert!(super::extract_slug(&p).is_none());
    }
}
