use anyhow::Result;
use std::io::{self, Write};

/// Print the initialization script for `.zshrc`.
///
/// This command embeds the `assets/init.zsh` file at compile time using
/// [`include_str!`] and writes it to `stdout`.
///
/// # Notes for developers
/// - The output is intended to be used like:
///   ```zsh
///   eval "$($(rz) init)"
///   ```
///   so escaping and quoting inside `init.zsh` must be correct.
/// - Do not modify this function to generate code dynamically.
///   Keeping the init script as a static asset makes behavior predictable.
/// - **Important:** `assets/init.zsh` must not use `local` variables,
///   because the script is injected directly into the user's interactive
///   shell via `eval`, and `local` would cause unexpected scoping issues.
///   Use `typeset -g` or global variables instead.
/// - The script is written directly to `stdout`, no additional formatting
///   or logging should be added.
///
/// # Errors
/// Returns an error if writing to `stdout` fails.
pub fn cmd_init() -> Result<()> {
    let s = include_str!("../assets/init.zsh");
    io::stdout().write_all(s.as_bytes())?;
    Ok(())
}
