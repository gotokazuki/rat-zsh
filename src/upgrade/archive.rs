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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn make_targz_with_single_file(name: &str, contents: &[u8]) -> NamedTempFile {
        use flate2::Compression;
        use flate2::write::GzEncoder;
        use tar::Builder;

        let tmp = tempfile::NamedTempFile::new().expect("targz temp");
        let gz = GzEncoder::new(
            std::fs::File::create(tmp.path()).expect("open gz out"),
            Compression::default(),
        );
        let mut tar = Builder::new(gz);

        let mut payload = tempfile::NamedTempFile::new().expect("payload temp");
        payload.write_all(contents).expect("write payload");

        tar.append_path_with_name(payload.path(), name)
            .expect("append to tar");
        tar.into_inner()
            .expect("finish tar")
            .finish()
            .expect("finish gz");

        tmp
    }

    #[test]
    fn sha256_file_returns_expected_digest() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(b"hello\n").unwrap();

        let got = sha256_file(f.path()).unwrap();
        let want = "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03";
        assert_eq!(got, want);
    }

    #[test]
    fn make_executable_sets_exec_bits_on_unix() {
        let f = tempfile::NamedTempFile::new().unwrap();
        make_executable(f.path()).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(f.path()).unwrap().permissions().mode();
            assert_ne!(mode & 0o111, 0, "executable bits should be set");
        }
    }

    #[test]
    fn extract_if_archive_finds_rz_and_returns_tempfile() {
        let tgz = make_targz_with_single_file("rz", b"dummy-binary");
        let extracted = extract_if_archive(tgz.path()).expect("extract ok");

        let mut buf = Vec::new();
        std::fs::File::open(extracted.path())
            .unwrap()
            .read_to_end(&mut buf)
            .unwrap();
        assert_eq!(buf, b"dummy-binary");
    }

    #[test]
    fn extract_if_archive_errors_when_rz_not_present() {
        let tgz = make_targz_with_single_file("foo", b"not rz");
        let err = extract_if_archive(tgz.path()).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("archive does not contain rz binary"),
            "unexpected error: {msg}"
        );
    }
}
