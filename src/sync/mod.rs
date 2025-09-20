mod cleanup;
mod jobs;
mod progress;
mod resolve;

use anyhow::{Context, Result};
use indicatif::{MultiProgress, ProgressBar};
use rayon::prelude::*;
use std::fs;
use std::time::Duration;

use crate::config::load_config;
use crate::git::ensure_repo;
use crate::paths::paths;
use crate::sync::cleanup::{cleanup_stale_plugins, cleanup_stale_repos};

use progress::{err_style, ok_style, spinner_style};
use resolve::{resolve_source_file, symlink};

/// Synchronize plugins defined in `config.toml`.
///
/// High-level flow:
/// 1. Ensure directory layout under `~/.rz` (`bin/`, `plugins/`, `repos/`, and the parent of `config.toml`).
/// 2. Load configuration and build a list of jobs to run (see [`jobs::build_jobs`]).
/// 3. Run clone/fetch + link resolution **in parallel** with progress spinners.
///    - For `source`-type plugins: resolve the source file inside the repo (or use `file` hint) and symlink it.
///    - For `fpath`-type plugins: symlink the **directory** so it is appended to `fpath`.
/// 4. Clean up stale plugin links and repositories that are no longer referenced (see [`cleanup`]).
///
/// Progress reporting uses `indicatif::MultiProgress`; each job gets its own spinner.  
/// Errors in individual jobs are captured and shown on the job’s line; processing continues for the rest.
pub fn cmd_sync() -> Result<()> {
    let p = paths()?;
    fs::create_dir_all(&p.bin)?;
    fs::create_dir_all(&p.plugins)?;
    fs::create_dir_all(&p.repos)?;
    if let Some(parent) = p.config.parent() {
        fs::create_dir_all(parent)?;
    }

    let cfg = load_config()?;
    if cfg.plugins.is_empty() {
        eprintln!("no plugins in {}", p.config.display());
        return Ok(());
    }

    let (jobs, expect_plugin_names, expect_repo_slugs) = jobs::build_jobs(&cfg, &p);

    let mp = MultiProgress::new();
    let run_style = spinner_style();
    let done_style = ok_style();
    let fail_style = err_style();

    let mut bars: Vec<ProgressBar> = Vec::with_capacity(jobs.len());
    for j in &jobs {
        let pb = mp.add(ProgressBar::new_spinner());
        pb.set_style(run_style.clone());
        pb.set_message(format!("syncing {}", j.display));
        pb.enable_steady_tick(Duration::from_millis(80));
        bars.push(pb);
    }

    jobs.par_iter().enumerate().for_each(|(idx, job)| {
        let pb = &bars[idx];
        let res: Result<()> = (|| {
            ensure_repo(&job.url, &job.repo_dir, job.rev.as_deref())?;

            if job.link_path.exists() {
                let _ = fs::remove_file(&job.link_path);
            }
            if job.kind_fpath {
                symlink(&job.repo_dir, &job.link_path)?;
            } else {
                let src = resolve_source_file(&job.repo_dir, job.file_hint.as_deref())
                    .with_context(|| {
                        format!("no source file found in {}", job.repo_dir.display())
                    })?;
                symlink(&src, &job.link_path)?;
            }
            Ok(())
        })();

        match res {
            Ok(_) => {
                pb.set_style(done_style.clone());
                pb.finish();
            }
            Err(e) => {
                pb.set_style(fail_style.clone());
                pb.finish_with_message(format!("syncing {} (error: {})", job.display, e));
            }
        }
    });

    cleanup_stale_plugins(&mp, &p.plugins, &expect_plugin_names)?;
    cleanup_stale_repos(&mp, &p.repos, &expect_repo_slugs, &p.plugins)?;

    Ok(())
}
