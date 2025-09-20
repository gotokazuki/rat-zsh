use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;

use crate::paths::paths;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub plugins: Vec<Plugin>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Plugin {
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub repo: String,
    #[serde(default)]
    pub rev: Option<String>,
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

pub fn load_config() -> Result<Config> {
    let p = paths()?;
    let txt = fs::read_to_string(&p.config)
        .with_context(|| format!("config not found: {}", p.config.display()))?;
    let cfg: Config = toml::from_str(&txt).context("failed to parse config.toml")?;
    Ok(cfg)
}

pub fn cmd_list() -> Result<()> {
    let cfg = load_config()?;
    for pl in cfg.plugins {
        let t = pl.r#type.as_deref().unwrap_or("source");
        let display = pl.name.as_deref().unwrap_or(&pl.repo);
        println!("- {} ({}) [{}]", display, pl.source, t);
    }
    Ok(())
}
