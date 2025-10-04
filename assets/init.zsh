# rat-zsh init
if [[ -z "${_RZ_INIT:-}" ]]; then
  typeset -g _RZ_INIT=1
  typeset -g RZ_HOME="${XDG_CONFIG_HOME:-$HOME}/.rz"
  typeset -g RZ_BIN="$RZ_HOME/bin"
  typeset -g RZ_PLUGINS="$RZ_HOME/plugins"

  export PATH="$RZ_BIN:$PATH"
  fpath=("$RZ_PLUGINS"/*(N-/) $fpath)

  # block list (plus any dot-starting directory)
  typeset -a _rz_block=(
    docs doc examples example samples sample
    tests test spec scripts script tools bin
    assets images img node_modules
  )

  # return success if the dir has at least one file starting with "_"
  _rz_looks_like_completion_dir() {
    typeset dir=$1
    typeset -a _rz_matches
    _rz_matches=("$dir"/_*(N.))
    (( ${#_rz_matches} > 0 ))
  }

  # print candidate fpath dirs for a resolved plugin target
  _rz_fpath_dirs_for_target() {
    typeset target=$1
    typeset -a _rz_out
    _rz_out=()

    # plugin root itself?
    if _rz_looks_like_completion_dir "$target"; then
      _rz_out+=("$target")
    fi

    # scan children (exclude dot-dirs and blocked names)
    for s in "$target"/*(N-/); do
      typeset name=${s:t}
      [[ $name == .* ]] && continue
      if (( ${_rz_block[(Ie)$name]} )); then
        continue
      fi
      if _rz_looks_like_completion_dir "$s"; then
        _rz_out+=("$s")
      fi
    done

    print -rl -- $_rz_out
  }

  # build final fpath additions from plugins
  typeset -a _rz_fp_acc
  _rz_fp_acc=()
  for p in "$RZ_PLUGINS"/*(N@-/); do
    typeset target="${p:A}"   # absolute + symlink-resolved
    _rz_fp_acc+=($(_rz_fpath_dirs_for_target "$target"))
  done
  if (( ${#_rz_fp_acc} )); then
    fpath=($_rz_fp_acc $fpath)
  fi

  # Initialize completion system (must be after fpath is set)
  autoload -Uz compinit
  if [[ -z "${_RZ_COMPINIT_DONE:-}" ]]; then
    typeset -g _RZ_COMPINIT_DONE=1
    compinit -u
  fi

  typeset -a _rz_tail_slugs=(
    zsh-users__zsh-autosuggestions
    zsh-users__zsh-syntax-highlighting
  )

  typeset -a _rz_normal _rz_tail
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

  for p in $_rz_normal; do
    if [[ -L "$p" && -f "$p" ]]; then source "$p"; continue; fi
    case "$p" in
      *.zsh|*.plugin.zsh|*.zsh-theme) source "$p" ;;
    esac
  done

  typeset s="" q="" slug="" target=""
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