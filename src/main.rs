mod asciicast;
mod pty;
mod recorder;
use anyhow::{anyhow, Result};
use std::env;

fn main() -> Result<()> {
    let path = env::args()
        .nth(1)
        .ok_or(anyhow!("output filename missing"))?;

    let mut recorder = recorder::new(path, recorder::Format::Asciicast, false, true)?;
    pty::exec(&["/bin/bash"], &mut recorder)?;

    Ok(())
}
