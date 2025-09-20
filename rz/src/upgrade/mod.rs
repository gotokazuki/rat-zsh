mod archive;
mod github;

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::paths::paths;
use archive::{extract_if_archive, make_executable, sha256_file};
use github::{candidate_asset_names, download_to_temp, fetch_latest_release, gh_client};

#[cfg(windows)]
fn target_bin_path(bin_dir: &Path) -> std::path::PathBuf {
    bin_dir.join("rz.exe")
}
#[cfg(not(windows))]
fn target_bin_path(bin_dir: &Path) -> std::path::PathBuf {
    bin_dir.join("rz")
}

pub fn cmd_upgrade() -> Result<()> {
    let p = paths()?;
    fs::create_dir_all(&p.bin)?;
    let target_bin = target_bin_path(&p.bin);

    let client = gh_client()?;
    let rel = fetch_latest_release(&client)?;
    let tag = rel.tag_name.as_str();

    let cands = candidate_asset_names(tag)?;
    let chosen = cands
        .iter()
        .find_map(|want| rel.assets.iter().find(|a| a.name == *want));
    let asset = if let Some(a) = chosen {
        a
    } else {
        rel.assets.first().context("no assets in latest release")?
    };

    eprintln!("downloading {}", asset.name);
    let downloaded = download_to_temp(&client, &asset.browser_download_url)
        .with_context(|| format!("failed to download: {}", asset.browser_download_url))?;

    let bin_path = extract_if_archive(downloaded.path(), &p.bin)?;
    atomic_replace(&bin_path, &target_bin)?;
    eprintln!("upgraded to {}", rel.tag_name);

    if bin_path != downloaded.path() {
        let _ = fs::remove_file(&bin_path);
    }
    Ok(())
}

fn atomic_replace(src: &Path, dst: &Path) -> Result<()> {
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
            eprintln!("already up-to-date");
            return Ok(());
        }
    }
    #[cfg(windows)]
    {
        let _ = fs::remove_file(dst);
    }
    fs::rename(&tmp_dst, dst)?;
    Ok(())
}
