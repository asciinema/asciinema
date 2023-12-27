use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Config {
    server: Server,
    api: Api,
    cmd: Cmd,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Server {
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Api {
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Cmd {
    rec: Rec,
    play: Play,
}

#[derive(Debug, Deserialize, Default)]
#[allow(unused)]
pub struct Rec {
    pub input: bool,
    pub command: Option<String>,
    pub env: String,
    pub idle_time_limit: Option<f64>,
    pub prefix_key: Option<String>,
    pub pause_key: String,
    pub add_marker_key: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Play {
    pub speed: f64,
    pub idle_time_limit: Option<f64>,
    pub pause_key: String,
    pub step_key: String,
    pub next_marker_key: String,
}

impl Config {
    pub fn new(server_url: Option<String>) -> Result<Self> {
        let user_config_file = home()?.join("config.toml");

        let mut config = config::Config::builder()
            .set_default("server.url", None::<Option<String>>)?
            .set_default("api.url", None::<Option<String>>)?
            .set_default("cmd.rec.input", false)?
            .set_default("cmd.rec.env", "SHELL,TERM")?
            .set_default("cmd.rec.pause_key", "C-\\")?
            .set_default("cmd.play.speed", 1.0)?
            .set_default("cmd.play.pause_key", " ")?
            .set_default("cmd.play.step_key", ".")?
            .set_default("cmd.play.next_marker_key", "]")?
            .add_source(
                config::File::with_name(&user_config_file.to_string_lossy()).required(false),
            )
            .add_source(config::Environment::with_prefix("asciinema").separator("_"));

        if let Some(url) = server_url {
            config = config.set_override("server.url", Some(url))?;
        }

        Ok(config.build()?.try_deserialize()?)
    }

    pub fn server_url(&self) -> Option<&String> {
        self.server.url.as_ref().or(self.api.url.as_ref())
    }
}

pub fn home() -> Result<PathBuf> {
    env::var("ASCIINEMA_CONFIG_HOME")
        .map(PathBuf::from)
        .or(env::var("XDG_CONFIG_HOME").map(|home| Path::new(&home).join("asciinema")))
        .or(env::var("HOME").map(|home| Path::new(&home).join(".config").join("asciinema")))
        .map_err(|_| anyhow!("need $HOME or $XDG_CONFIG_HOME or $ASCIINEMA_CONFIG_HOME"))
}
