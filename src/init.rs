use anyhow::Result;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::config::load_config;
use crate::paths::paths;

/// Escape a path for safe inclusion in a Zsh double-quoted string.
fn zsh_quote_path(p: &str) -> String {
    // Escape backslashes and double-quotes for zsh double-quoted context.
    p.replace('\\', "\\\\").replace('"', "\\\"")
}

pub fn cmd_init() -> Result<()> {
    let cfg = load_config()?;
    let p = paths()?; // has .repos, .plugins, .bin, etc.

    // Collect absolute fpath dirs from config-driven plugins (type = "fpath")
    let mut fpath_dirs: Vec<String> = Vec::new();

    for pl in &cfg.plugins {
        let ty = pl.r#type.as_deref().unwrap_or("source");
        if ty != "fpath" {
            continue;
        }
        // slug = owner/repo -> owner__repo
        let slug = pl.repo.replace('/', "__");
        let root = p.repos.join(&slug);

        // If plugin root is missing, skip (user may not have synced yet)
        if !root.is_dir() {
            continue;
        }

        for d in &pl.fpath_dirs {
            let cand: PathBuf = {
                let pd = Path::new(d);
                if pd.is_absolute() {
                    pd.to_path_buf()
                } else {
                    root.join(pd)
                }
            };
            if cand.is_dir() {
                // Prefer canonical path; fall back to raw if canonicalize fails.
                let canon = std::fs::canonicalize(&cand).unwrap_or(cand);
                let s = canon.to_string_lossy().to_string();
                fpath_dirs.push(s);
            }
        }
    }

    // Sort + dedup to stabilize output
    fpath_dirs.sort();
    fpath_dirs.dedup();

    // Build Zsh snippet that prepends fpath entries (if any)
    let fpath_snippet = if fpath_dirs.is_empty() {
        // No fpath entries — emit only a comment (harmless)
        String::from("  # no fpath entries from config.toml\n")
    } else {
        let quoted: Vec<String> = fpath_dirs
            .iter()
            .map(|s| format!("\"{}\"", zsh_quote_path(s)))
            .collect();
        format!(
            "  # fpath entries from config.toml\n  fpath=({} $fpath)\n",
            quoted.join(" ")
        )
    };

    // Render final init.zsh (template with {FPATh_SNIPPET} placeholder)
    let script = INIT_ZSH_TEMPLATE.replace("{FPATh_SNIPPET}", &fpath_snippet);

    io::stdout().write_all(script.as_bytes())?;
    Ok(())
}

/// Static init.zsh template.
/// NOTE:
/// - {FPATh_SNIPPET} will be replaced at runtime with computed fpath lines.
/// - Avoid `local` in this script (it is eval'ed into user's interactive shell).
const INIT_ZSH_TEMPLATE: &str = r#"# rat-zsh init
if [[ -z "${_RZ_INIT:-}" ]]; then
  typeset -g _RZ_INIT=1
  typeset -g RZ_HOME="${XDG_CONFIG_HOME:-$HOME}/.rz"
  typeset -g RZ_BIN="$RZ_HOME/bin"
  typeset -g RZ_PLUGINS="$RZ_HOME/plugins"

  # Prepend rz bin to PATH
  export PATH="$RZ_BIN:$PATH"

{FPATh_SNIPPET}
  # Initialize completion system (must be after fpath is set)
  autoload -Uz compinit
  if [[ -z "${_RZ_COMPINIT_DONE:-}" ]]; then
    typeset -g _RZ_COMPINIT_DONE=1
    compinit -u
  fi

  # Source-order management (tail plugins last)
  typeset -a _rz_tail_slugs=(
    zsh-users__zsh-autosuggestions
    zsh-users__zsh-syntax-highlighting
  )

  typeset -a _rz_normal _rz_tail
  _rz_normal=()
  _rz_tail=()

  # Classify plugin entries under $RZ_PLUGINS
  typeset p target slug
  for p in "$RZ_PLUGINS"/*(N@-); do
    target="${p:A}"
    slug=""
    if [[ $target == */repos/* ]]; then
      slug="${${target##*/repos/}%%/*}"
    fi
    typeset -i is_tail=0
    for s in $_rz_tail_slugs; do
      if [[ $slug == $s ]]; then is_tail=1; break; fi
    done
    if (( is_tail )); then _rz_tail+=("$p"); else _rz_normal+=("$p"); fi
  done

  # Source normal plugins
  for p in $_rz_normal; do
    if [[ -L "$p" && -f "$p" ]]; then source "$p"; continue; fi
    case "$p" in
      *.zsh|*.plugin.zsh|*.zsh-theme) source "$p" ;;
    esac
  done

  # Source tail plugins in fixed order
  typeset q
  for s in $_rz_tail_slugs; do
    for q in $_rz_tail; do
      target="${q:A}"
      slug=""
      [[ $target == */repos/* ]] && slug="${${target##*/repos/}%%/*}"
      if [[ $slug == $s ]]; then
        if [[ -L "$q" && -f "$q" ]]; then source "$q"; continue; fi
        case "$q" in
          *.zsh|*.plugin.zsh|*.zsh-theme) source "$q" ;;
        esac
      fi
    done
  done
fi
"#;
