<h1>
<p align="center">
<img src="../images/icon/rat-zsh.png" alt="rat-zsh icon" width="256" /><br />
Rat Zsh
</p>
</h1>

**Rat Zsh** is a minimal and lightweight **plugin manager for zsh**.  
It is entirely implemented in zsh and only depends on git.

- 🚀 Installation with a single `curl` line
- ⚙️ Configuration file in TOML (`$(rz home)/config.toml`)
- 🧩 Plugins are fetched from GitHub repositories
- 📦 Just add one `eval` line in `.zshrc` to start using it

## Installation

```bash
curl -fsSL https://raw.githubusercontent.com/gotokazuki/rat-zsh/main/install.zsh | zsh
```

This installs `$(rz home)/bin/rz`.  
(Default install location is `$RAT_ZSH_HOME`. If not set, it falls back to `$ZDOTDIR/.rz`, and if not set, to `$HOME/.rz`.)

## Configuration

Write your plugin configuration in `$(rz home)/config.toml`.  
A sample file will be created on the first install.

```toml
[[plugins]]
source = "github"
repo   = "zsh-users/zsh-autosuggestions"
type   = "source"
file   = "zsh-autosuggestions.zsh"

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
```

### Supported keys

| Key    | Description                                     |
|--------|-------------------------------------------------|
| source | Plugin source (currently only `github` is supported) |
| repo   | Repository in `owner/repo` format               |
| rev    | Optional. Fix to a tag or branch                |
| file   | Optional. Relative path to the file to `source` |
| type   | `"source"` or `"fpath"` (default: `"source"`)   |
| name   | Optional. Alias name for the plugin             |

### Multiple plugins from a single repository

Some repositories provide multiple plugins (e.g. ohmyzsh/ohmyzsh).
In such cases you must specify the file and a unique name so they don’t overwrite each other:

```toml
[[plugins]]
source = "github"
repo   = "ohmyzsh/ohmyzsh"
type   = "source"
file   = "lib/clipboard.zsh"
name   = "clipboard"

[[plugins]]
source = "github"
repo   = "ohmyzsh/ohmyzsh"
type   = "source"
file   = "plugins/copypath/copypath.plugin.zsh"
name   = "copypath"
```

Here:

- file selects the plugin file inside the repository.
- name defines how it will appear in `~/.rz/plugins/` and in the log messages.

## Plugin load order

By default, rat-zsh loads plugins in alphabetical order, except it enforces the following rule:

- all other plugins
- `zsh-users/zsh-autosuggestions`
- `zsh-users/zsh-syntax-highlighting`

rat-zsh handles this automatically:
all other plugins are sourced first, then `zsh-autosuggestions`, and finally `zsh-syntax-highlighting`.

You can check the effective order with:

```zsh
rz order
```

## Setting up `.zshrc`

Add the following line to your `~/.zshrc`:

```zsh
eval "$("${RAT_ZSH_HOME:-${ZDOTDIR:-$HOME}/.rz}/bin/rz" init)"
```

## Commands

```zsh
rz init   # Print initialization code for .zshrc
rz sync   # Clone/update plugins defined in config.toml
rz list   # List parsed plugins
rz home   # Show RAT_ZSH_HOME
rz order  # Show the effective plugin load order
```

## Update

```zsh
rz sync
```

This updates both plugins and rat-zsh itself.  
Internally, rat-zsh pulls the latest changes from its own GitHub repository along with plugin updates.

## Uninstall

```zsh
rm -rf "$(rz home)"
```

Then remove the line `eval "$("$(rz home)/bin/rz" init)"` from `.zshrc`.

## Recommended setting

### Speeding up zsh-autosuggestions

By default, `zsh-autosuggestions` rebinds zle widgets before every prompt,  
which may slow down input responsiveness in some environments.

Add the following environment variable **before `rz init` in your .zshrc**  
to disable automatic rebinding and improve performance:

```zsh
# .zshrc
export ZSH_AUTOSUGGEST_MANUAL_REBIND=1
eval "$("${RAT_ZSH_HOME:-${ZDOTDIR:-$HOME}/.rz}/bin/rz" init)"
```

- Effect: Skips redundant processing at every prompt, resulting in a snappier shell.
- Note: If you add plugins or change key bindings later, you may need to run `zle autosuggest-start` manually.
- Especially effective when `zsh-autosuggestions` is loaded last (rat-zsh ensures this automatically).
