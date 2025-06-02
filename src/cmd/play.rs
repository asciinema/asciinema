use anyhow::Result;

use crate::asciicast;
use crate::cli;
use crate::config::{self, Config};
use crate::player::{self, KeyBindings};
use crate::status;
use crate::tty;
use crate::util;

impl cli::Play {
    pub fn run(self) -> Result<()> {
        let config = Config::new(None)?;
        let speed = self.speed.or(config.playback.speed).unwrap_or(1.0);
        let idle_time_limit = self.idle_time_limit.or(config.playback.idle_time_limit);

        status::info!("Replaying session from {}", self.file);

        let path = util::get_local_path(&self.file)?;
        let keys = get_key_bindings(&config.playback)?;

        let ended = loop {
            let recording = asciicast::open_from_path(&*path)?;
            let tty = tty::DevTty::open()?;

            let ended = player::play(
                recording,
                tty,
                speed,
                idle_time_limit,
                self.pause_on_markers,
                &keys,
                self.resize,
            )?;

            if !self.loop_ || !ended {
                break ended;
            }
        };

        if ended {
            status::info!("Playback ended");
        } else {
            status::info!("Playback interrupted");
        }

        Ok(())
    }
}

fn get_key_bindings(config: &config::Playback) -> Result<KeyBindings> {
    let mut keys = KeyBindings::default();

    if let Some(key) = config.pause_key()? {
        keys.pause = key;
    }

    if let Some(key) = config.step_key()? {
        keys.step = key;
    }

    if let Some(key) = config.next_marker_key()? {
        keys.next_marker = key;
    }

    Ok(keys)
}
