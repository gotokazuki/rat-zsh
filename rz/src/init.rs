use anyhow::Result;

pub fn cmd_init() -> Result<()> {
    println!(
        r#"# rat-zsh init
if [[ -z "${{_RZ_INIT:-}}" ]]; then
  typeset -g _RZ_INIT=1
  typeset -g RZ_HOME="${{XDG_CONFIG_HOME:-$HOME}}/.rz"
  typeset -g RZ_BIN="$RZ_HOME/bin"
  typeset -g RZ_PLUGINS="$RZ_HOME/plugins"

  export PATH="$RZ_BIN:$PATH"
  fpath=("$RZ_PLUGINS" $fpath)

  autoload -Uz compinit
  if [[ -z "${{_RZ_COMPINIT_DONE:-}}" ]]; then
    typeset -g _RZ_COMPINIT_DONE=1
    compinit -u
  fi

  typeset -a _rz_tail_slugs=(
    zsh-users__zsh-autosuggestions
    zsh-users__zsh-syntax-highlighting
  )

  typeset -a _rz_normal _rz_tail
  for p in "$RZ_PLUGINS"/*(N@-); do
    target="${{p:A}}"
    slug=""
    if [[ $target == */repos/* ]]; then
      slug="${{${{target##*/repos/}}%%/*}}"
    fi
    typeset -i is_tail=0
    for s in $_rz_tail_slugs; do
      if [[ $slug == $s ]]; then is_tail=1; break; fi
    done
    if (( is_tail )); then _rz_tail+=("$p"); else _rz_normal+=("$p"); fi
  done

  for p in $_rz_normal; do
    if [[ -L "$p" && -f "$p" ]]; then source "$p"; continue; fi
    case "$p" in
      *.zsh|*.plugin.zsh|*.zsh-theme) source "$p" ;;
    esac
  done

  typeset s="" q="" slug="" target=""
  for s in $_rz_tail_slugs; do
    for q in $_rz_tail; do
      target="${{q:A}}"
      slug=""
      [[ $target == */repos/* ]] && slug="${{${{target##*/repos/}}%%/*}}"
      if [[ $slug == $s ]]; then
        if [[ -L "$q" && -f "$q" ]]; then source "$q"; continue; fi
        case "$q" in
          *.zsh|*.plugin.zsh|*.zsh-theme) source "$q" ;;
        esac
      fi
    done
  done
fi
"#
    );
    Ok(())
}
