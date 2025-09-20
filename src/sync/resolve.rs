use anyhow::{Result, anyhow};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

/// Create a symbolic link from `src` to `dst`.
///
/// This implementation is Unix-only. On non-Unix systems,
/// additional handling would be required.
pub fn symlink(src: &Path, dst: &Path) -> Result<()> {
    use std::os::unix::fs::symlink;
    symlink(src, dst).map_err(Into::into)
}

/// Resolve the actual plugin source file within a repository.
///
/// Search order:
/// 1. If `hint` is provided and points to an existing file, use it.
/// 2. Otherwise, try to find a file matching one of these patterns:
///    - `*.plugin.zsh`
///    - `*.zsh`
///    - `*.zsh-theme`
///
/// Returns:
/// - The first file that matches.
/// - Error if no valid file is found.
pub fn resolve_source_file(repo_dir: &Path, hint: Option<&str>) -> Result<PathBuf> {
    if let Some(rel) = hint {
        let p = repo_dir.join(rel);
        if p.is_file() {
            return Ok(p);
        }
    }
    for pat in ["*.plugin.zsh", "*.zsh", "*.zsh-theme"] {
        if let Some(p) = glob1(repo_dir, pat) {
            return Ok(p);
        }
    }
    Err(anyhow!("no plugin file matched"))
}

/// Find the first file in a directory that matches a glob-like pattern.
/// Only supports simple wildcards (`*`) and dots (`.`).
fn glob1(dir: &Path, pat: &str) -> Option<PathBuf> {
    let re = glob_to_regex(pat);
    fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .map(|s| re.is_match(s))
                .unwrap_or(false)
        })
}

/// Convert a minimal glob pattern into a regular expression.
/// Supported:
/// - `*` → `.*`
/// - `.` → escaped as `\.`
///
/// Other characters are copied literally.
fn glob_to_regex(pat: &str) -> Regex {
    let mut s = String::from("^");
    for ch in pat.chars() {
        match ch {
            '*' => s.push_str(".*"),
            '.' => s.push_str("\\."),
            c => s.push(c),
        }
    }
    s.push('$');
    Regex::new(&s).unwrap()
}
