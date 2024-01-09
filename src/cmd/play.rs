use crate::{player, tty};
use anyhow::Result;
use clap::Args;
use std::fs;

#[derive(Debug, Args)]
pub struct Cli {
    filename: String,

    /// Limit idle time to a given number of seconds
    #[arg(short, long, value_name = "SECS")]
    idle_time_limit: Option<f64>,

    /// Set playback speed
    #[arg(short, long)]
    speed: Option<f64>,

    /// Loop loop loop loop
    #[arg(short, long, name = "loop")]
    loop_: bool,

    /// Automatically pause on markers
    #[arg(short = 'm', long)]
    pause_on_markers: bool,
}

impl Cli {
    pub fn run(self) -> Result<()> {
        let speed = self.speed.unwrap_or(1.0);

        loop {
            let file = fs::File::open(&self.filename)?;
            let tty = tty::DevTty::open()?;

            player::play(
                file,
                tty,
                speed,
                self.idle_time_limit,
                self.pause_on_markers,
            )?;

            if !self.loop_ {
                break;
            }
        }

        Ok(())
    }
}
