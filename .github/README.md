<h1>
<p align="center">
<img src="../assets/icons/rat-zsh@4x.png" alt="rat-zsh icon" width="128" /><br />
Rat Zsh
</p>
</h1>
<!-- markdownlint-disable MD013 -->
<p align="center">
  <a href="https://github.com/gotokazuki/rat-zsh/actions/workflows/lint.yml"><img src="https://github.com/gotokazuki/rat-zsh/actions/workflows/lint.yml/badge.svg?branch=main" alt="Lint Status" /></a>
  <a href="https://github.com/gotokazuki/rat-zsh/actions/workflows/test.yml"><img src="https://github.com/gotokazuki/rat-zsh/actions/workflows/test.yml/badge.svg?branch=main" alt="Test Status" /></a>
  <a href="https://github.com/gotokazuki/rat-zsh/releases/latest"><img src="https://img.shields.io/github/v/release/gotokazuki/rat-zsh?logo=github&label=release" alt="Latest Release" /></a>
</p>
<!-- markdownlint-enable MD013 -->

<p align="center">
A lightweight, fast, and reproducible plugin manager for zsh.<br>
Made with 🐭 & 🦀 — no magic, no heavy frameworks.
</p>

## Features 🐭✨

- 🚀 Simple setup
  - Install with a single `curl` line
  - Just add one `eval` line in `.zshrc` to start using it
- ⚙️ Configurable and reproducible
  - Simple TOML-based configuration
  - Automatic plugin load order control
- 🐙 GitHub integration
  - Fetches plugins from GitHub repositories
  - Supports branches, tags, and commits
  - Handles Git submodules automatically
- ⚡️ Lightweight and fast
  - Parallel plugin sync
  - Built in Rust 🦀
- 🔄 Seamless updates
  - Self-upgrade
  - Plugin sync

## Installation

```zsh
curl -fsSL https://raw.githubusercontent.com/gotokazuki/rat-zsh/main/install.zsh | zsh
```

This installs `$(rz home)/bin/rz`.  
(Default install location is `$XDG_CONFIG_HOME/.rz`. If not set, it falls back to `$HOME/.rz`.)

## Setting up `.zshrc`

Add the following line to your `.zshrc`:

```zsh
eval "$("${XDG_CONFIG_HOME:-$HOME}"/.rz/bin/rz init)"
```

## Configuration

Write your plugin configuration in `$(rz home)/config.toml`.  
A sample file will be created on the first install.

```toml
[[plugins]]
source = "github"
repo = "zsh-users/zsh-autosuggestions"
type = "source"
file = "zsh-autosuggestions.zsh"

[[plugins]]
source = "github"
repo = "zsh-users/zsh-completions"
type = "fpath"
fpath_dirs = ["src"]

[[plugins]]
source = "github"
repo = "zsh-users/zsh-syntax-highlighting"
type = "source"
file = "zsh-syntax-highlighting.zsh"

[[plugins]]
source = "github"
repo = "zsh-users/zsh-history-substring-search"
type = "source"
file = "zsh-history-substring-search.zsh"

[[plugins]]
source = "github"
repo = "olets/zsh-abbr"
type = "source"
file = "zsh-abbr.zsh"

[[plugins]]
source = "github"
repo = "gotokazuki/rat-zsh"
type = "fpath"
fpath_dirs = ["contrib/completions/zsh"]
```

### Supported keys

| Key          | Description |
|--------------|-------------|
| source       | Plugin source (currently only `github` is supported) |
| repo         | Repository in `owner/repo` format |
| rev          | Optional. Pin to a tag or branch |
| file         | Optional. Relative path to the file to `source` |
| type         | `"source"` or `"fpath"` (default: `"source"`) |
| name         | Optional. Alias name for the plugin |
| fpath_dirs   | Optional. List of directories (relative to the repo root) to include in `$fpath` when `type = "fpath"` |

### Example: fpath_dirs usage

Some plugins (like `zsh-completions` or `rat-zsh` itself) include completion functions in nested directories.  
You can explicitly define which directories should be added to Zsh’s `$fpath`:

```toml
[[plugins]]
source = "github"
repo = "gotokazuki/rat-zsh"
type = "fpath"
fpath_dirs = ["contrib/completions/zsh"]

[[plugins]]
source = "github"
repo = "zsh-users/zsh-completions"
type = "fpath"
fpath_dirs = ["src"]
```

When you run `rz list`, these appear like:

```bash
fpath
- gotokazuki/rat-zsh (github) [fpath: contrib/completions/zsh] @main (4243bb3)
- zsh-users/zsh-completions (github) [fpath: src] @master (173a14c)
```

### Multiple plugins from a single repository

Some repositories provide multiple plugins (e.g. ohmyzsh/ohmyzsh).  
In such cases you must specify the file and a unique name so they don’t overwrite each other:

```toml
[[plugins]]
source = "github"
repo = "ohmyzsh/ohmyzsh"
type = "source"
file = "lib/clipboard.zsh"
name = "clipboard"

[[plugins]]
source = "github"
repo = "ohmyzsh/ohmyzsh"
type = "source"
file = "plugins/copypath/copypath.plugin.zsh"
name = "copypath"
```

In this example:

- `file` selects the plugin file inside the repository.
- `name` defines how it will appear in `$(rz home)/plugins/` and in the log messages.

### Pinning to a specific tag or branch

You can use `rev` to pin a plugin to a specific version (tag or branch).  
This ensures reproducible environments and avoids unexpected updates.

```toml
# Pin to a tag
[[plugins]]
source = "github"
repo = "zsh-users/zsh-autosuggestions"
rev = "v0.7.0"
type = "source"
file = "zsh-autosuggestions.zsh"

# Pin to a branch
[[plugins]]
source = "github"
repo = "zsh-users/zsh-completions"
rev = "develop"
type = "fpath"
fpath_dirs = ["src"]
```

- If rev is not specified, the plugin is synced to the default branch (usually main or master).
- When rev is specified, Rat Zsh checks out that branch or tag after cloning or fetching.
- Tags are checked out in a detached state.
- Branches are checked out in an attached state and will track updates.

## Plugin load order

By default, Rat Zsh loads plugins in the following order:

| Priority | Plugins                              |
|----------|--------------------------------------|
| 1        | All other plugins (alphabetical)     |
| 2        | zsh-users/zsh-autosuggestions        |
| 3        | zsh-users/zsh-syntax-highlighting    |

This order is enforced automatically — you don’t need to configure it manually.

To see the actual order on your system:

```zsh
rz list
```

Example output:

```zsh
Source order
- olets/zsh-abbr (github) [source] @main (13b34cd)
- zsh-users/zsh-history-substring-search (github) [source] @master (87ce96b)
- zsh-users/zsh-autosuggestions (github) [source] @master (85919cd)
- zsh-users/zsh-syntax-highlighting (github) [source] @master (5eb677b)

fpath
- gotokazuki/rat-zsh (github) [fpath: contrib/completions/zsh] @main (4243bb3)
- zsh-users/zsh-completions (github) [fpath: src] @master (173a14c)
```

## Commands

```zsh
rz init     # Print initialization code for .zshrc
rz sync     # Clone/update plugins defined in config.toml
rz upgrade  # Update rat-zsh itself to the latest release
rz list     # Show plugins in the effective load order with source/type metadata
rz home     # Show the rz home directory
```

## Update

Update Rat Zsh itself:

```zsh
rz upgrade
```

Update plugins:

```zsh
rz sync
```

## Uninstall

```zsh
rm -rf "$(rz home)"
```

Then remove the line `eval "$("${XDG_CONFIG_HOME:-$HOME}/.rz/bin/rz" init)"` from `.zshrc`.

## Recommended configuration

### Speeding up zsh-autosuggestions

By default, `zsh-autosuggestions` rebinds zle widgets before every prompt,  
which may slow down input responsiveness in some environments.

Add the following environment variable **before `rz init` in your .zshrc**  
to disable automatic rebinding and improve performance:

```zsh
# .zshrc
export ZSH_AUTOSUGGEST_MANUAL_REBIND=1
eval "$("${XDG_CONFIG_HOME:-$HOME}/.rz/bin/rz" init)"
```

- Effect: Skips redundant processing at every prompt, resulting in a snappier shell.
- Note: If you add plugins or change key bindings later, you may need to run `zle autosuggest-start` manually.
- Especially effective when `zsh-autosuggestions` is loaded last (rat-zsh ensures this automatically).

## License

MIT License. See [LICENSE](../LICENSE) for details.
