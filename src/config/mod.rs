use anyhow::{Context, Result};
use std::env;
use std::os::unix::process::CommandExt;
use std::process::Command;

use crate::paths::paths;

pub fn cmd_config() -> Result<()> {
    let p = paths()?;
    let config_path = p.config;

    // Determine editor
    // 1. EDITOR env var
    // 2. default "vim"
    let editor_env = env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());

    let mut cmd = Command::new(&editor_env);
    cmd.arg(&config_path);

    // If the editor is vim (case-insensitive check on the binary name), add -n to avoid swap files
    let is_vim = std::path::Path::new(&editor_env)
        .file_name()
        .map(|name| name.to_string_lossy().to_lowercase().contains("vim"))
        .unwrap_or(false);

    if is_vim {
        cmd.arg("-n");
    }

    // Replace current process with editor
    let err = cmd.exec();
    Err(err).context(format!("failed to launch editor: {}", editor_env))
}
