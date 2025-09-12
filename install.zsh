#!/usr/bin/env zsh
set -e

RAT_ZSH_HOME="${RAT_ZSH_HOME:-${ZDOTDIR:-$HOME}/.rz}"
BIN_DIR="$RAT_ZSH_HOME/bin"
PLUGINS_DIR="$RAT_ZSH_HOME/plugins"
REPOS_DIR="$RAT_ZSH_HOME/repos"
CONFIG="$RAT_ZSH_HOME/config.toml"

mkdir -p "$BIN_DIR" "$PLUGINS_DIR" "$REPOS_DIR"

# download rz
RZ_URL="https://raw.githubusercontent.com/gotokazuki/rat-zsh/main/rz"
curl -fsSL "$RZ_URL" -o "$BIN_DIR/rz"
chmod +x "$BIN_DIR/rz"

# generate config.toml if not exists
if [[ ! -f "$CONFIG" ]]; then
  cat > "$CONFIG" <<'EOF'
# ~/.rz/config.toml (rat-zsh)
[[plugins]]
source = "github"
repo   = "zsh-users/zsh-autosuggestions"
type   = "source"
file   = "zsh-autosuggestions.zsh"
name   = "zz-autosuggestions"

[[plugins]]
source = "github"
repo   = "zsh-users/zsh-completions"
type   = "fpath"

[[plugins]]
source = "github"
repo   = "zsh-users/zsh-syntax-highlighting"
type   = "source"
file   = "zsh-syntax-highlighting.zsh"

[[plugins]]
source = "github"
repo   = "zsh-users/zsh-history-substring-search"
type   = "source"
file   = "zsh-history-substring-search.zsh"

[[plugins]]
source = "github"
repo   = "olets/zsh-abbr"
type   = "source"
file   = "zsh-abbr.zsh"
EOF
  echo "Wrote sample config: $CONFIG"
fi

$BIN_DIR/rz sync

echo "rat-zsh installed to: $BIN_DIR/rz"
echo "Add this line to your .zshrc if not present:"
echo '  eval "$("$HOME/.rz/bin/rz" init)"'