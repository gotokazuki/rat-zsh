use anyhow::{Result, anyhow};
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

pub fn sha256_file(path: &Path) -> Result<String> {
    let mut f = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(unix)]
pub fn make_executable(p: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perm = fs::metadata(p)?.permissions();
    perm.set_mode(0o755);
    fs::set_permissions(p, perm)?;
    Ok(())
}
#[cfg(not(unix))]
pub fn make_executable(_p: &Path) -> Result<()> {
    Ok(())
}

pub fn extract_if_archive(temp_path: &Path) -> Result<PathBuf> {
    let fname = temp_path.file_name().and_then(|s| s.to_str()).unwrap_or("");

    if fname.ends_with(".tar.gz") || fname.ends_with(".tgz") {
        let dir = tempdir()?;
        let f = fs::File::open(temp_path)?;
        let gz = GzDecoder::new(f);
        let mut ar = tar::Archive::new(gz);

        let want = if cfg!(windows) { "rz.exe" } else { "rz" };
        let out = dir.path().join(want);

        let mut found = false;
        for entry in ar.entries()? {
            let mut e = entry?;
            let path = e.path()?;
            if let Some(name) = path.file_name().and_then(|s| s.to_str())
                && (name == want || name == "rz")
            {
                let mut of = fs::File::create(&out)?;
                std::io::copy(&mut e, &mut of)?;
                found = true;
                break;
            }
        }

        if !found {
            return Err(anyhow!("archive does not contain rz binary"));
        }
        Ok(out)
    } else {
        Ok(temp_path.to_path_buf())
    }
}
