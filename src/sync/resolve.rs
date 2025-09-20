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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Read;
    use tempfile::tempdir;

    #[test]
    fn glob_to_regex_works_for_basic_patterns() {
        let re = glob_to_regex("*.zsh");
        assert!(re.is_match("foo.zsh"));
        assert!(!re.is_match("foo.zsh-theme"));

        let re2 = glob_to_regex("*.plugin.zsh");
        assert!(re2.is_match("bar.plugin.zsh"));
        assert!(!re2.is_match("bar.zsh"));
    }

    #[test]
    fn resolve_uses_hint_when_valid() {
        let tmp = tempdir().unwrap();
        let repo = tmp.path();

        let hinted = repo.join("subdir").join("my.zsh");
        fs::create_dir_all(hinted.parent().unwrap()).unwrap();
        fs::write(&hinted, "# hint").unwrap();

        let got = resolve_source_file(repo, Some("subdir/my.zsh")).unwrap();
        assert_eq!(got, hinted);
    }

    #[test]
    fn resolve_falls_back_to_plugin_then_zsh_then_theme() {
        let tmp = tempdir().unwrap();
        let repo = tmp.path();

        let f_theme = repo.join("t.zsh-theme");
        let f_zsh = repo.join("b.zsh");
        let f_plug = repo.join("a.plugin.zsh");

        fs::write(&f_theme, "# theme").unwrap();
        let got1 = resolve_source_file(repo, None).unwrap();
        assert_eq!(got1, f_theme);

        fs::write(&f_zsh, "# zsh").unwrap();
        let got2 = resolve_source_file(repo, None).unwrap();
        assert_eq!(got2, f_zsh);

        fs::write(&f_plug, "# plugin").unwrap();
        let got3 = resolve_source_file(repo, None).unwrap();
        assert_eq!(got3, f_plug);
    }

    #[test]
    fn resolve_errors_when_nothing_matches() {
        let tmp = tempdir().unwrap();
        let repo = tmp.path();

        let err = resolve_source_file(repo, None).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("no plugin file matched"));
    }

    #[test]
    fn symlink_creates_link_and_points_to_src() {
        let tmp = tempdir().unwrap();
        let dir = tmp.path();

        let src = dir.join("src.zsh");
        let dst = dir.join("link.zsh");

        fs::write(&src, "echo ok\n").unwrap();
        symlink(&src, &dst).unwrap();

        let md = fs::symlink_metadata(&dst).unwrap();
        assert!(md.file_type().is_symlink());

        let target = fs::read_link(&dst).unwrap();
        let target_abs = if target.is_absolute() {
            target
        } else {
            dst.parent().unwrap().join(target)
        };
        assert_eq!(
            fs::canonicalize(target_abs).unwrap(),
            fs::canonicalize(&src).unwrap()
        );

        let mut s = String::new();
        fs::File::open(&dst)
            .unwrap()
            .read_to_string(&mut s)
            .unwrap();
        assert!(s.contains("echo ok"));
    }
}
