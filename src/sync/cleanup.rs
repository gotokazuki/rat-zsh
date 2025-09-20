use anyhow::Result;
use indicatif::{MultiProgress, ProgressBar};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path};
use std::time::Duration;

use super::progress::{err_style, ok_style, spinner_style};

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
