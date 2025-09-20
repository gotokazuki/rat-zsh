use std::collections::HashSet;
use std::path::PathBuf;

use crate::config::Config;
use crate::paths::Paths;

/// Represents a single plugin synchronization job.
///
/// Each job corresponds to one entry in `config.toml` and contains all
/// the information needed to clone/update the repository and create the
/// appropriate symlink in the `plugins` directory.
#[derive(Clone)]
pub struct SyncJob {
    pub display: String,
    pub url: String,
    pub repo_dir: PathBuf,
    pub link_path: PathBuf,
    pub kind_fpath: bool,
    pub file_hint: Option<String>,
    pub rev: Option<String>,
}

/// Build synchronization jobs from the parsed configuration.
///
/// This function converts `Config.plugins` into a list of [`SyncJob`]s,
/// while also computing the expected plugin names and repository slugs.
/// These are later used for cleanup (removing stale plugins/repos).
///
/// # Arguments
/// - `cfg`: The loaded configuration (`config.toml`).
/// - `p`: Paths struct containing important directories (`bin`, `repos`, `plugins`, etc.).
///
/// # Returns
/// A tuple of:
/// - `Vec<SyncJob>`: List of jobs to execute during sync.
/// - `HashSet<String>`: Expected plugin names (for symlinks).
/// - `HashSet<String>`: Expected repo slugs (for cloned repos).
pub fn build_jobs(cfg: &Config, p: &Paths) -> (Vec<SyncJob>, HashSet<String>, HashSet<String>) {
    let mut expect_plugin_names = HashSet::new();
    let mut expect_repo_slugs = HashSet::new();
    let mut jobs: Vec<SyncJob> = Vec::new();

    for pl in &cfg.plugins {
        if pl.repo.trim().is_empty() {
            continue;
        }
        let slug = pl.repo.replace('/', "__");
        let repo_dir = p.repos.join(&slug);
        let plug_name = pl.name.as_deref().unwrap_or(&slug);
        let link = p.plugins.join(plug_name);

        expect_plugin_names.insert(plug_name.to_string());
        expect_repo_slugs.insert(slug.clone());

        let url = match pl.source.as_str() {
            "" | "github" => format!("https://github.com/{}.git", pl.repo),
            other => other.to_string(),
        };

        jobs.push(SyncJob {
            display: pl.name.clone().unwrap_or_else(|| pl.repo.clone()),
            url,
            repo_dir,
            link_path: link,
            kind_fpath: pl.r#type.as_deref() == Some("fpath"),
            file_hint: pl.file.clone(),
            rev: pl.rev.clone(),
        });
    }

    (jobs, expect_plugin_names, expect_repo_slugs)
}
