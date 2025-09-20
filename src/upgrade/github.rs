use anyhow::{Result, bail};
use reqwest::blocking::Client;
use reqwest::blocking::Response;
use reqwest::header::{ACCEPT, HeaderMap, HeaderValue, USER_AGENT};
use serde::Deserialize;
use std::env;
use tempfile::NamedTempFile;

/// Representation of a GitHub release response.
/// Contains the tag name and associated release assets.
#[derive(Debug, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub assets: Vec<Asset>,
}

/// Representation of a single GitHub release asset.
/// Includes the filename and the download URL.
#[derive(Debug, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
}

/// Create a GitHub API client with default headers.
///
/// - Adds `Accept` and `User-Agent` headers (required by GitHub API).
/// - If `GITHUB_TOKEN` is set in the environment, adds an Authorization header.
///
/// # Errors
/// - Returns an error if the client cannot be built.
/// - Returns an error if the token is invalid for the header.
pub fn gh_client() -> Result<Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/vnd.github+json"),
    );
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("gotokazuki-rz-upgrader"),
    );
    if let Ok(tok) = env::var("GITHUB_TOKEN") {
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {}", tok))?,
        );
    }
    let client = Client::builder().default_headers(headers).build()?;
    Ok(client)
}

/// Fetch metadata for the latest release from GitHub.
///
/// Uses the `/releases/latest` API endpoint to retrieve the release tag
/// and asset list.
///
/// # Errors
/// - Returns an error if the request fails or the response cannot be parsed.
pub fn fetch_latest_release(client: &Client) -> Result<Release> {
    let url = "https://api.github.com/repos/gotokazuki/rat-zsh/releases/latest";
    let rel: Release = client.get(url).send()?.error_for_status()?.json()?;
    Ok(rel)
}

/// Generate candidate asset filenames for the current platform.
///
/// The naming convention is assumed to be:
/// `rz-<tag>-<os>-<arch>.tar.gz`
///
/// # Errors
/// - Returns an error if the current OS/arch is unsupported.
pub fn candidate_asset_names(tag: &str) -> Result<Vec<String>> {
    let (os, arch) = detect_target()?;
    Ok(vec![format!("rz-{}-{}-{}.tar.gz", tag, os, arch)])
}

/// Detect the current OS and architecture using Rust’s compile-time constants.
///
/// # Returns
/// - `"linux"` or `"macos"`
/// - `"x86_64"` or `"aarch64"`
///
/// # Errors
/// - Returns an error if the OS or architecture is not supported.
fn detect_target() -> Result<(&'static str, &'static str)> {
    let os = match std::env::consts::OS {
        "linux" => "linux",
        "macos" => "macos",
        other => bail!("unsupported OS: {}", other),
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" | "arm64" => "aarch64",
        other => bail!("unsupported ARCH: {}", other),
    };
    Ok((os, arch))
}

/// Download a file from GitHub to a temporary file.
///
/// The temporary file will have a `.tar.gz` suffix so it can be
/// properly identified and handled later.
///
/// # Errors
/// - Returns an error if the request fails.
/// - Returns an error if writing to the temporary file fails.
pub fn download_to_temp(client: &Client, url: &str) -> Result<NamedTempFile> {
    let mut resp: Response = client.get(url).send()?.error_for_status()?;
    let tmp = tempfile::Builder::new().suffix(".tar.gz").tempfile()?;
    std::io::copy(&mut resp, &mut tmp.as_file())?;
    Ok(tmp)
}
