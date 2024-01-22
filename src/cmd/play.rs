use crate::config::Config;
use crate::format::asciicast;
use crate::logger;
use crate::player::{self, KeyBindings};
use crate::tty;
use anyhow::{anyhow, Result};
use clap::Args;
use reqwest::Url;
use std::io;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

#[derive(Debug, Args)]
pub struct Cli {
    #[arg(value_name = "FILENAME_OR_URL")]
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
        let idle_time_limit = self.idle_time_limit.or(config.cmd_play_idle_time_limit());

        logger::info!("Replaying session from {}", self.filename);

        let path = get_path(&self.filename)?;

        let ended = loop {
            let recording = asciicast::open_from_path(&path)?;
            let tty = tty::DevTty::open()?;
            let keys = get_key_bindings(config)?;

            let ended = player::play(
                recording,
                tty,
                speed,
                idle_time_limit,
                self.pause_on_markers,
                &keys,
            )?;

            if !self.loop_ {
                break ended;
            }
        };

        if ended {
            logger::info!("Playback ended");
        } else {
            logger::info!("Playback interrupted");
        }

        Ok(())
    }
}

enum LocalPath {
    Normal(PathBuf),
    Temporary(NamedTempFile),
}

impl AsRef<Path> for LocalPath {
    fn as_ref(&self) -> &Path {
        match self {
            LocalPath::Normal(p) => p,
            LocalPath::Temporary(f) => f.path(),
        }
    }
}

fn get_path(filename: &str) -> Result<LocalPath> {
    if filename.starts_with("https://") || filename.starts_with("http://") {
        download_asciicast(filename)
            .map(LocalPath::Temporary)
            .map_err(|e| anyhow!("download failed: {e}"))
    } else {
        Ok(LocalPath::Normal(PathBuf::from(filename)))
    }
}

fn download_asciicast(url: &str) -> Result<NamedTempFile> {
    let mut response = reqwest::blocking::get(Url::parse(url)?)?;
    response.error_for_status_ref()?;
    let mut file = NamedTempFile::new()?;
    io::copy(&mut response, &mut file)?;

    Ok(file)
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
