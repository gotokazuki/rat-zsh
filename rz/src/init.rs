use anyhow::Result;
use std::io::{self, Write};

pub fn cmd_init() -> Result<()> {
    let s = include_str!("../assets/init.zsh");
    io::stdout().write_all(s.as_bytes())?;
    Ok(())
}
