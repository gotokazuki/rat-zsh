use anyhow::{Result, anyhow};
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::Path;
use tempfile::NamedTempFile;

/// Compute the SHA-256 checksum of a file.
///
/// Reads the file in 8 KB chunks and updates the hasher incrementally.
/// Returns the hex-encoded digest as a `String`.
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

/// Mark a file as executable on Unix-like systems.
///
/// Sets permissions to `0o755` (rwxr-xr-x).
/// On non-Unix platforms, this function would need an alternative implementation.
pub fn make_executable(p: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perm = fs::metadata(p)?.permissions();
    perm.set_mode(0o755);
    fs::set_permissions(p, perm)?;
    Ok(())
}

/// Extract the `rz` binary from a `.tar.gz` archive.
///
/// - Opens the provided `temp_path` as a gzip-compressed tar archive.
/// - Iterates over all entries until a file named exactly `"rz"` is found.
/// - Copies that entry into a new `NamedTempFile` with a `rz-` prefix.
/// - Returns the temporary file containing the extracted binary.
///
/// # Errors
/// - If no `rz` binary is found in the archive, returns an error.
/// - If the file cannot be opened, decompressed, or written, returns an error.
pub fn extract_if_archive(temp_path: &Path) -> Result<NamedTempFile> {
    let f = fs::File::open(temp_path)?;
    let gz = GzDecoder::new(f);
    let mut ar = tar::Archive::new(gz);

    for entry in ar.entries()? {
        let mut e = entry?;
        let path = e.path()?;
        if let Some(name) = path.file_name().and_then(|s| s.to_str())
            && name == "rz"
        {
            let mut tmp = tempfile::Builder::new().prefix("rz-").tempfile()?;
            std::io::copy(&mut e, tmp.as_file_mut())?;
            return Ok(tmp);
        }
    }

    Err(anyhow!("archive does not contain rz binary"))
}
