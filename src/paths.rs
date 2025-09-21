use anyhow::Result;
use std::{env, ffi::OsString, path::PathBuf};

/// Holds important directory paths used by rat-zsh.
///
/// - `bin`: directory where the `rz` binary is placed
/// - `plugins`: directory where plugin files are stored
/// - `repos`: directory where plugin repositories are cloned
/// - `config`: path to the `config.toml` configuration file
#[derive(Clone)]
pub struct Paths {
    pub bin: PathBuf,
    pub plugins: PathBuf,
    pub repos: PathBuf,
    pub config: PathBuf,
}

/// Compute the rat-zsh home directory from the given environment variables.
///
/// Behavior:
/// - If `xdg` is set, base is `<xdg>`.
/// - Otherwise, base is `<home>`.
///
/// In both cases, `".rz"` is appended at the end.
fn rz_home_from_env(xdg: Option<OsString>, home: Option<OsString>) -> PathBuf {
    let base = xdg
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(home.unwrap_or_default()));
    base.join(".rz")
}

/// Return the rat-zsh home directory based on the current process environment.
///
/// Resolution order:
/// 1. If `$XDG_CONFIG_HOME` is set, use `$XDG_CONFIG_HOME/.rz`.
/// 2. Otherwise, use `$HOME/.rz`.
pub fn rz_home() -> Result<PathBuf> {
    Ok(rz_home_from_env(
        env::var_os("XDG_CONFIG_HOME"),
        env::var_os("HOME"),
    ))
}

/// Returns a `Paths` struct with the resolved directories used by rat-zsh.
///
/// This includes:
/// - `bin` (`rz_home()/bin`)
/// - `plugins` (`rz_home()/plugins`)
/// - `repos` (`rz_home()/repos`)
/// - `config` (`rz_home()/config.toml`)
pub fn paths() -> Result<Paths> {
    let home = rz_home()?;
    Ok(Paths {
        bin: home.join("bin"),
        plugins: home.join("plugins"),
        repos: home.join("repos"),
        config: home.join("config.toml"),
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use tempfile::tempdir;

    use crate::paths::Paths;

    fn paths_under(home: &Path) -> Paths {
        Paths {
            bin: home.join("bin"),
            plugins: home.join("plugins"),
            repos: home.join("repos"),
            config: home.join("config.toml"),
        }
    }

    #[test]
    fn rz_home_prefers_xdg_when_present() {
        let xdg = tempdir().unwrap();
        let home = tempdir().unwrap();

        let got = super::rz_home_from_env(Some(xdg.path().into()), Some(home.path().into()));
        assert_eq!(got, xdg.path().join(".rz"));
    }

    #[test]
    fn rz_home_falls_back_to_home() {
        let home = tempdir().unwrap();
        let got = super::rz_home_from_env(None, Some(home.path().into()));
        assert_eq!(got, home.path().join(".rz"));
    }

    #[test]
    fn paths_under_builds_expected() {
        let base = tempdir().unwrap();
        let p = paths_under(&base.path().join(".rz"));
        assert_eq!(p.bin, base.path().join(".rz/bin"));
        assert_eq!(p.plugins, base.path().join(".rz/plugins"));
        assert_eq!(p.repos, base.path().join(".rz/repos"));
        assert_eq!(p.config, base.path().join(".rz/config.toml"));
    }
}
