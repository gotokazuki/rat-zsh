use anyhow::Result;
use std::{env, path::PathBuf};

#[derive(Clone)]
pub struct Paths {
    pub bin: PathBuf,
    pub plugins: PathBuf,
    pub repos: PathBuf,
    pub config: PathBuf,
}

pub fn rz_home() -> Result<PathBuf> {
    let xdg = env::var_os("XDG_CONFIG_HOME");
    let base = xdg
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(".config"));
    Ok(base.join(".rz"))
}

pub fn paths() -> Result<Paths> {
    let home = rz_home()?;
    Ok(Paths {
        bin: home.join("bin"),
        plugins: home.join("plugins"),
        repos: home.join("repos"),
        config: home.join("config.toml"),
    })
}
