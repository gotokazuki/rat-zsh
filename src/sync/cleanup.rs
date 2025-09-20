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
