use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;

use crate::settings::Config;
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

fn command_exists(cmd: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {}", cmd))
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
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
        if !pl.requires.is_empty() {
            let missing: Vec<_> = pl
                .requires
                .iter()
                .filter(|cmd| !command_exists(cmd))
                .collect();

            if !missing.is_empty() {
                eprintln!(
                    "\x1b[33mskip plugin {}: missing required commands {:?}\x1b[0m",
                    pl.name.as_deref().unwrap_or(&pl.repo),
                    missing
                );
                continue;
            }
        }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Plugin;
    use std::collections::HashSet;
    use tempfile::tempdir;

    fn make_paths(tmp: &std::path::Path) -> Paths {
        Paths {
            bin: tmp.join("bin"),
            plugins: tmp.join("plugins"),
            repos: tmp.join("repos"),
            config: tmp.join("config.toml"),
        }
    }

    #[test]
    fn build_jobs_basic_and_github_url_and_flags() {
        let tmp = tempdir().unwrap();
        let p = make_paths(tmp.path());

        let cfg = Config {
            plugins: vec![
                Plugin {
                    source: "".into(),
                    repo: "zsh-users/zsh-autosuggestions".into(),
                    rev: None,
                    file: Some("zsh-autosuggestions.zsh".into()),
                    r#type: None,
                    name: None,
                    fpath_dirs: Vec::new(),
                    requires: Vec::new(),
                },
                Plugin {
                    source: "github".into(),
                    repo: "zsh-users/zsh-completions".into(),
                    rev: None,
                    file: None,
                    r#type: Some("fpath".into()),
                    name: Some("comps".into()),
                    fpath_dirs: Vec::new(),
                    requires: Vec::new(),
                },
                Plugin {
                    source: "github".into(),
                    repo: "olets/zsh-abbr".into(),
                    rev: Some("v1.2.3".into()),
                    file: Some("zsh-abbr.zsh".into()),
                    r#type: None,
                    name: None,
                    fpath_dirs: Vec::new(),
                    requires: Vec::new(),
                },
            ],
        };

        let (jobs, expect_names, expect_slugs) = build_jobs(&cfg, &p);

        assert_eq!(jobs.len(), 3);

        let j0 = &jobs[0];
        assert_eq!(j0.display, "zsh-users/zsh-autosuggestions");
        assert_eq!(
            j0.url,
            "https://github.com/zsh-users/zsh-autosuggestions.git"
        );
        assert_eq!(j0.repo_dir, p.repos.join("zsh-users__zsh-autosuggestions"));
        assert_eq!(
            j0.link_path,
            p.plugins.join("zsh-users__zsh-autosuggestions")
        );
        assert!(!j0.kind_fpath);
        assert_eq!(j0.file_hint.as_deref(), Some("zsh-autosuggestions.zsh"));
        assert!(j0.rev.is_none());

        let j1 = &jobs[1];
        assert_eq!(j1.display, "comps");
        assert_eq!(j1.url, "https://github.com/zsh-users/zsh-completions.git");
        assert!(j1.kind_fpath);
        assert_eq!(j1.file_hint, None);
        assert_eq!(j1.link_path, p.plugins.join("comps"));

        let j2 = &jobs[2];
        assert_eq!(j2.display, "olets/zsh-abbr");
        assert_eq!(j2.rev.as_deref(), Some("v1.2.3"));
        assert_eq!(j2.file_hint.as_deref(), Some("zsh-abbr.zsh"));

        let want_names: HashSet<_> = ["zsh-users__zsh-autosuggestions", "comps", "olets__zsh-abbr"]
            .into_iter()
            .map(str::to_string)
            .collect();
        let want_slugs: HashSet<_> = [
            "zsh-users__zsh-autosuggestions",
            "zsh-users__zsh-completions",
            "olets__zsh-abbr",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();

        assert_eq!(expect_names, want_names);
        assert_eq!(expect_slugs, want_slugs);
    }

    #[test]
    fn build_jobs_skips_empty_repo_entries() {
        let tmp = tempdir().unwrap();
        let p = make_paths(tmp.path());

        let cfg = Config {
            plugins: vec![
                Plugin {
                    source: "github".into(),
                    repo: "".into(),
                    rev: None,
                    file: None,
                    r#type: None,
                    name: None,
                    fpath_dirs: Vec::new(),
                    requires: Vec::new(),
                },
                Plugin {
                    source: "github".into(),
                    repo: "owner/repo".into(),
                    rev: None,
                    file: None,
                    r#type: None,
                    name: None,
                    fpath_dirs: Vec::new(),
                    requires: Vec::new(),
                },
            ],
        };

        let (jobs, expect_names, expect_slugs) = build_jobs(&cfg, &p);

        assert_eq!(jobs.len(), 1);
        assert_eq!(expect_names.len(), 1);
        assert_eq!(expect_slugs.len(), 1);

        assert_eq!(jobs[0].display, "owner/repo");
        assert_eq!(jobs[0].url, "https://github.com/owner/repo.git");
        assert_eq!(jobs[0].repo_dir, p.repos.join("owner__repo"));
        assert_eq!(jobs[0].link_path, p.plugins.join("owner__repo"));
    }
}
