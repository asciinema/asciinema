use crate::config::Config;
use crate::{
    player::{self, KeyBindings},
    tty,
};
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
    pub fn run(self, config: &Config) -> Result<()> {
        let speed = self.speed.or(config.cmd_play_speed()).unwrap_or(1.0);

        loop {
            let file = fs::File::open(&self.filename)?;
            let tty = tty::DevTty::open()?;
            let keys = get_key_bindings(config)?;

            player::play(
                file,
                tty,
                speed,
                self.idle_time_limit,
                self.pause_on_markers,
                &keys,
            )?;

            if !self.loop_ {
                break;
            }
        }

        Ok(())
    }
}

fn get_key_bindings(config: &Config) -> Result<KeyBindings> {
    let mut keys = KeyBindings::default();

    if let Some(key) = config.cmd_play_pause_key()? {
        keys.pause = key;
    }

    if let Some(key) = config.cmd_play_step_key()? {
        keys.step = key;
    }

    if let Some(key) = config.cmd_play_next_marker_key()? {
        keys.next_marker = key;
    }

    Ok(keys)
}
