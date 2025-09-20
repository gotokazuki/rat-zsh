use anyhow::Result;
use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, HeaderMap, HeaderValue, USER_AGENT};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub assets: Vec<Asset>,
}
#[derive(Debug, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
}

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

pub fn fetch_latest_release(client: &Client) -> Result<Release> {
    let url = "https://api.github.com/repos/gotokazuki/rat-zsh/releases/latest";
    let rel: Release = client.get(url).send()?.error_for_status()?.json()?;
    Ok(rel)
}

pub fn candidate_asset_names(tag: &str) -> Vec<String> {
    let (os, arch, _exe) = detect_target();
    vec![
        format!("rz-{}-{}-{}.tar.gz", tag, os, arch),
        format!("rz-{}-{}-{}.zip", tag, os, arch),
    ]
}

fn detect_target() -> (&'static str, &'static str, &'static str) {
    let os = match std::env::consts::OS {
        "linux" => "linux",
        "macos" => "macos",
        "windows" => "windows",
        _ => "linux",
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" | "arm64" => "aarch64",
        _ => "x86_64",
    };
    let exe = if os == "windows" { ".exe" } else { "" };
    (os, arch, exe)
}

use reqwest::blocking::Response;
use tempfile::NamedTempFile;

pub fn download_to_temp(client: &Client, url: &str) -> Result<NamedTempFile> {
    let mut resp: Response = client.get(url).send()?.error_for_status()?;
    let mut tmp = NamedTempFile::new()?;
    std::io::copy(&mut resp, &mut tmp)?;
    Ok(tmp)
}
