use crate::config::load_config;
use crate::git::{UpdateStatus, attached_update_status, is_dirty_repo_root};
use crate::paths::paths;
use anyhow::Result;
use colored::Colorize;
use order::{PluginEntry, resolve_order};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

mod fs_scan;
mod order;

#[derive(Clone)]
struct Meta {
    source: String,
    ty: String,
    name: Option<String>,
    fpath_dirs: Vec<String>,
    rev: Option<String>,
}

#[derive(Debug, Clone)]
struct GitRev {
    head_short: String,
    head_ref: Option<String>,
}

#[derive(Debug, Clone)]
pub enum RevKind {
    Branch { name: String },
    Tag { name: String },
    Detached,
}

#[derive(Debug, Clone, Default)]
pub struct RevParts {
    pub role_label: &'static str,
    pub kind: Option<RevKind>,
    pub commit_short: Option<String>,
    pub fpath_dirs: Vec<String>,
}

fn resolve_git_dir(repo_root: &Path) -> Option<PathBuf> {
    let dotgit = repo_root.join(".git");
    if dotgit.is_dir() {
        return Some(dotgit);
    }
    if dotgit.is_file()
        && let Ok(s) = fs::read_to_string(&dotgit)
        && let Some(rest) = s.strip_prefix("gitdir:")
    {
        let raw = rest.trim();
        let p = Path::new(raw);
        let abs = if p.is_absolute() {
            p.to_path_buf()
        } else {
            repo_root.join(p)
        };
        return Some(abs);
    }
    None
}

fn read_to_string_lossy(p: &Path) -> Option<String> {
    fs::read_to_string(p).ok().map(|mut s| {
        if let Some(pos) = s.find('\n') {
            s.truncate(pos);
        }
        s.trim().to_string()
    })
}

fn resolve_ref_sha(git_dir: &Path, refname: &str) -> Option<String> {
    let ref_path = git_dir.join(refname);
    if let Some(s) = read_to_string_lossy(&ref_path)
        && s.len() >= 40
    {
        return Some(s);
    }
    let packed = git_dir.join("packed-refs");
    if let Ok(content) = fs::read_to_string(packed) {
        for line in content.lines() {
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            if let Some((sha, name)) = line.split_once(' ')
                && name.trim() == refname
                && sha.len() >= 40
            {
                return Some(sha.trim().to_string());
            }
        }
    }
    None
}

fn short7(sha: &str) -> String {
    sha.chars().take(7).collect()
}

fn git_rev_for_repo(repo_root: &Path) -> Option<GitRev> {
    let git_dir = resolve_git_dir(repo_root)?;
    let head = read_to_string_lossy(&git_dir.join("HEAD"))?;

    if let Some(refname) = head.strip_prefix("ref: ").map(|s| s.to_string())
        && let Some(sha) = resolve_ref_sha(&git_dir, &refname)
    {
        return Some(GitRev {
            head_short: short7(&sha),
            head_ref: Some(refname),
        });
    }

    if head.len() >= 7 {
        let sha = head;
        return Some(GitRev {
            head_short: short7(&sha),
            head_ref: None,
        });
    }

    None
}

fn is_hex_sha(s: &str) -> bool {
    let s = s.trim();
    (7..=40).contains(&s.len()) && s.chars().all(|c| c.is_ascii_hexdigit())
}

fn rev_parts_for_repo(role: &'static str, cfg_rev: Option<&String>, repo_root: &Path) -> RevParts {
    if let Some(info) = git_rev_for_repo(repo_root) {
        if let Some(r) = cfg_rev {
            if is_hex_sha(r) {
                return RevParts {
                    role_label: role,
                    kind: Some(RevKind::Detached),
                    commit_short: Some(short7(r)),
                    ..Default::default()
                };
            }
            return RevParts {
                role_label: role,
                kind: Some(RevKind::Branch { name: r.clone() }),
                commit_short: Some(info.head_short),
                ..Default::default()
            };
        }
        if let Some(rf) = info.head_ref {
            if let Some(name) = rf.strip_prefix("refs/heads/") {
                return RevParts {
                    role_label: role,
                    kind: Some(RevKind::Branch {
                        name: name.to_string(),
                    }),
                    commit_short: Some(info.head_short),
                    ..Default::default()
                };
            }
            if let Some(name) = rf.strip_prefix("refs/tags/") {
                return RevParts {
                    role_label: role,
                    kind: Some(RevKind::Tag {
                        name: name.to_string(),
                    }),
                    commit_short: Some(info.head_short),
                    ..Default::default()
                };
            }
            return RevParts {
                role_label: role,
                kind: Some(RevKind::Detached),
                commit_short: Some(info.head_short),
                ..Default::default()
            };
        }
        return RevParts {
            role_label: role,
            kind: Some(RevKind::Detached),
            commit_short: Some(info.head_short),
            ..Default::default()
        };
    }

    RevParts {
        role_label: role,
        kind: cfg_rev.cloned().map(|r| RevKind::Branch { name: r }),
        commit_short: None,
        ..Default::default()
    }
}

fn fmt_kind(kind: &Option<RevKind>) -> String {
    match kind {
        Some(RevKind::Branch { name }) => format!("@{}", name).green().to_string(),
        Some(RevKind::Tag { name }) => format!("@{}", name).yellow().to_string(),
        Some(RevKind::Detached) => "@detached".red().to_string(),
        None => "".to_string(),
    }
}

fn fmt_commit(commit_short: &Option<String>) -> String {
    commit_short
        .as_ref()
        .map(|s| format!("({})", s).bright_black().to_string())
        .unwrap_or_default()
}

fn fmt_role(role: &str) -> String {
    format!("[{}]", role).bright_black().to_string()
}

/// Print plugins in their effective load order, split by **source** and **fpath** roles.
///
/// This command merges information from:
/// - The on-disk `plugins/` directory (actual load order and presence)
/// - `config.toml` (metadata: `name`, `source`, `type`)
///
/// ### Output format
/// The output is split into two sections:
///
/// 1. **`Source order`**
///    - Shows plugins whose `type` is **not** `fpath`.
///    - Order is computed from the `plugins/` directory:
///      - “Normal” plugins sorted alphabetically by display name
///      - “Tail” plugins (e.g. `zsh-autosuggestions`, `zsh-syntax-highlighting`) appended last in a fixed order
///    - Each line includes the display name (or configured `name`), the `source` (e.g. `github`), and the literal tag `[source]`.
///
/// 2. **`fpath`**
///    - Shows plugins whose `type` **is** `fpath`.
///    - For each plugin, candidate completion directories under its repository are discovered
///      (ignoring dot-directories like `.git`, `.github`, etc. and obvious non-completion folders).
///    - If one directory is found, it is shown after `fpath:` as an absolute path.
///      If multiple are found, they are shown as `{dir1, dir2, ...}` to indicate search order.
///      If none are found, only `[fpath]` is printed without a path.
///
/// ### Example
/// ```text
/// Source order
/// - olets/zsh-abbr (github) [source]
/// - zsh-users/zsh-history-substring-search (github) [source]
/// - zsh-users/zsh-autosuggestions (github) [source]
/// - zsh-users/zsh-syntax-highlighting (github) [source]
///
/// fpath
/// - zsh-users/zsh-completions (github) [fpath: /home/user/.rz/plugins/zsh-users__zsh-completions/src]
/// ```
///
/// Notes:
/// - Entries lacking config metadata are still listed but without extra fields.
/// - “Tail” membership is determined by a fixed list and ensures those plugins load last.
///
/// # Errors
/// Returns an error if configuration loading fails or if plugin directory scanning fails.
pub fn cmd_list(check_update: bool) -> Result<()> {
    let cfg = load_config()?;
    let mut meta: HashMap<String, Meta> = HashMap::new();
    for pl in cfg.plugins {
        let slug = pl.repo.replace('/', "__");
        meta.insert(
            slug,
            Meta {
                source: pl.source.clone(),
                ty: pl.r#type.as_deref().unwrap_or("source").to_string(),
                name: pl.name.clone(),
                fpath_dirs: pl.fpath_dirs.clone(),
                rev: pl.rev.clone(),
            },
        );
    }

    let p = paths()?;
    let ordered: Vec<PluginEntry> = resolve_order()?;

    println!("{}", "Source order".bold());

    for e in &ordered {
        if let Some(m) = meta.get(&e.slug).cloned() {
            if m.ty == "fpath" {
                continue;
            }
            let shown = m.name.unwrap_or_else(|| e.display.clone());
            let repo_root = p.repos.join(&e.slug);
            let parts = rev_parts_for_repo("source", m.rev.as_ref(), &repo_root);

            let kind = fmt_kind(&parts.kind);
            let commit = fmt_commit(&parts.commit_short);
            let role = fmt_role(parts.role_label);

            let git_status = if check_update {
                match &parts.kind {
                    Some(RevKind::Branch { name }) => {
                        let st = attached_update_status(&repo_root, name);
                        fmt_update_suffix_from_status(&st)
                    }
                    Some(RevKind::Detached) | Some(RevKind::Tag { .. }) => {
                        if is_dirty_repo_root(&repo_root) {
                            format!(" {}", "*".red().bold())
                        } else {
                            String::new()
                        }
                    }
                    _ => String::new(),
                }
            } else {
                String::new()
            };

            println!(
                "{} {} ({}) {}{}{}{}",
                "-".dimmed(),
                shown.bold().white(),
                m.source.cyan(),
                role,
                if kind.is_empty() {
                    "".to_string()
                } else {
                    format!(" {}", kind)
                },
                if commit.is_empty() {
                    "".to_string()
                } else {
                    format!(" {}", commit)
                },
                git_status,
            );
        } else {
            println!("- {}", e.display);
        }
    }

    println!("\n{}", "fpath".bold());
    for e in &ordered {
        if e.slug.is_empty() {
            continue;
        }
        if let Some(m) = meta.get(&e.slug).cloned() {
            if m.ty != "fpath" {
                continue;
            }
            let shown = m.name.unwrap_or_else(|| e.display.clone());
            let dirs = fs_scan::fpath_dirs_from_config(&p.plugins, &e.slug, &m.fpath_dirs)?;
            let dir_suffix = fs_scan::format_fpath_dirs(&dirs);
            let dir_part = if dir_suffix.is_empty() {
                String::new()
            } else {
                format!("{} ", format!("[fpath: {}]", dir_suffix).bright_black())
            };

            let repo_root = p.repos.join(&e.slug);
            let mut parts = rev_parts_for_repo("fpath", m.rev.as_ref(), &repo_root);
            parts.fpath_dirs = dirs;

            let kind = fmt_kind(&parts.kind);
            let commit = fmt_commit(&parts.commit_short);

            let git_status = if check_update {
                match &parts.kind {
                    Some(RevKind::Branch { name }) => {
                        let st = attached_update_status(&repo_root, name);
                        fmt_update_suffix_from_status(&st)
                    }
                    Some(RevKind::Detached) | Some(RevKind::Tag { .. }) => {
                        if is_dirty_repo_root(&repo_root) {
                            format!(" {}", "*".red().bold())
                        } else {
                            String::new()
                        }
                    }
                    _ => String::new(),
                }
            } else {
                String::new()
            };

            println!(
                "{} {} ({}) {}{}{}{}",
                "-".dimmed(),
                shown.bold().white(),
                m.source.cyan(),
                dir_part,
                if kind.is_empty() {
                    "".to_string()
                } else {
                    kind.to_string()
                },
                if commit.is_empty() {
                    "".to_string()
                } else {
                    format!(" {}", commit)
                },
                git_status,
            );
        }
    }

    Ok(())
}

fn fmt_update_suffix_from_status(st: &UpdateStatus) -> String {
    if st.unknown {
        let mut parts: Vec<String> = vec![];
        if st.dirty {
            parts.push("*".red().bold().to_string());
        }
        parts.push("?".red().bold().to_string());
        return format!(" {}", parts.join(" "));
    }

    let plus = if st.behind == 0 {
        "↓".blue().to_string()
    } else {
        format!("↓{}", st.behind).blue().to_string()
    };

    let slash = "/".white().to_string();

    let minus = if st.ahead == 0 {
        "↑".red().bold().to_string()
    } else {
        format!("↑{}", st.ahead).red().bold().to_string()
    };

    let mut parts: Vec<String> = vec![format!("{}{}{}", plus, slash, minus)];

    if st.dirty {
        parts.push("*".red().bold().to_string());
    }

    format!(" {}", parts.join(" "))
}
