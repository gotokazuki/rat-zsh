use anyhow::{Result, anyhow};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

pub fn symlink(src: &Path, dst: &Path) -> Result<()> {
    use std::os::unix::fs::symlink;
    symlink(src, dst).map_err(Into::into)
}

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
