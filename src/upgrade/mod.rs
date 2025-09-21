mod archive;
mod github;

use anyhow::{Context, Result};
use indicatif::ProgressBar;
use std::path::Path;
use std::{fs, time::Duration};

use crate::progress::ok_style;
use crate::{paths::paths, progress::spinner_style};
use archive::{extract_if_archive, make_executable, sha256_file};
use github::{candidate_asset_names, download_to_temp, fetch_latest_release, gh_client};

/// Resolve the target binary path inside `bin_dir`.
/// Always returns `<bin_dir>/rz`.
fn target_bin_path(bin_dir: &Path) -> std::path::PathBuf {
    bin_dir.join("rz")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReplaceOutcome {
    Replaced,
    Unchanged,
}

/// Upgrade the `rz` binary to the latest GitHub release.
///
/// Process:
/// 1. Create `~/.rz/bin` directory if missing.
/// 2. Fetch the latest release metadata from GitHub API.
/// 3. Compare release tag with current `CARGO_PKG_VERSION`.
///    - If equal → print "already up to date" and exit.
/// 4. Find the matching release asset for this OS/arch.
///    - Uses `candidate_asset_names` to build expected names.
///    - Falls back to first asset if no exact match.
/// 5. Download the asset tarball.
/// 6. Extract the `rz` binary from archive.
/// 7. Atomically replace the old binary with the new one
///    (skip replacement if SHA-256 hash is unchanged).
pub fn cmd_upgrade() -> Result<()> {
    let p = paths()?;
    fs::create_dir_all(&p.bin)?;
    let target_bin = target_bin_path(&p.bin);

    let pb = ProgressBar::new_spinner();
    pb.set_style(spinner_style());
    pb.enable_steady_tick(Duration::from_millis(200));
    pb.set_message("resolving latest release…");

    let client = gh_client()?;
    let rel = fetch_latest_release(&client)?;

    let latest_version = rel.tag_name.trim_start_matches('v');
    let current_version = env!("CARGO_PKG_VERSION");
    if latest_version == current_version {
        pb.set_style(ok_style());
        pb.finish_with_message(format!("already up to date ({})", rel.tag_name));
        return Ok(());
    }

    let tag = rel.tag_name.as_str();
    pb.set_message(format!("choosing asset for {}", tag));

    let cands = candidate_asset_names(tag)?;
    let chosen = cands
        .iter()
        .find_map(|want| rel.assets.iter().find(|a| a.name == *want));
    let asset = if let Some(a) = chosen {
        a
    } else {
        rel.assets.first().context("no assets in latest release")?
    };

    pb.set_message(format!("downloading {}", asset.name));
    let downloaded = download_to_temp(&client, &asset.browser_download_url)
        .with_context(|| format!("failed to download: {}", asset.browser_download_url))?;

    pb.set_message("extracting package…");
    let extracted = extract_if_archive(downloaded.path())?;

    pb.set_message("installing rz…");
    match atomic_replace(extracted.path(), &target_bin)? {
        ReplaceOutcome::Unchanged => {
            pb.set_style(ok_style());
            pb.finish_with_message(format!("already up-to-date ({})", rel.tag_name));
        }
        ReplaceOutcome::Replaced => {
            pb.set_style(ok_style());
            pb.finish_with_message(format!("upgraded to {}", rel.tag_name));
        }
    }

    Ok(())
}

/// Replace the destination binary (`dst`) atomically with `src`.
///
/// Steps:
/// - Copy `src` to a temporary file `<dst>.new`.
/// - Mark it as executable.
/// - If `dst` already exists:
///   - Compare SHA-256 of old and new binaries.
///   - If hashes match → remove temp file and print "already up-to-date".
/// - Otherwise, rename temp file to overwrite `dst`.
fn atomic_replace(src: &Path, dst: &Path) -> Result<ReplaceOutcome> {
    let tmp_dst = dst.with_extension("new");
    if tmp_dst.exists() {
        let _ = fs::remove_file(&tmp_dst);
    }
    fs::copy(src, &tmp_dst)?;
    make_executable(&tmp_dst)?;
    if dst.exists() {
        let old = sha256_file(dst).unwrap_or_default();
        let new = sha256_file(&tmp_dst).unwrap_or_default();
        if old == new {
            let _ = fs::remove_file(&tmp_dst);
            return Ok(ReplaceOutcome::Unchanged);
        }
    }
    fs::rename(&tmp_dst, dst)?;
    Ok(ReplaceOutcome::Replaced)
}
