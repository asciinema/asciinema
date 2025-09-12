use std::env;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Result};
use config::{self, File};
use reqwest::Url;
use serde::Deserialize;
use uuid::Uuid;

use crate::status;

const DEFAULT_SERVER_URL: &str = "https://asciinema.org";
const INSTALL_ID_FILENAME: &str = "install-id";

pub type Key = Option<Vec<u8>>;

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Config {
    server: Server,
    pub session: Session,
    pub playback: Playback,
    pub notifications: Notifications,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Server {
    url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[allow(unused)]
pub struct Session {
    pub command: Option<String>,
    pub capture_input: bool,
    pub capture_env: Option<String>,
    pub idle_time_limit: Option<f64>,
    pub prefix_key: Option<String>,
    pub pause_key: Option<String>,
    pub add_marker_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(unused)]
pub struct Playback {
    pub speed: Option<f64>,
    pub idle_time_limit: Option<f64>,
    pub pause_key: Option<String>,
    pub step_key: Option<String>,
    pub next_marker_key: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Notifications {
    pub enabled: bool,
    pub command: Option<String>,
}

impl Config {
    pub fn new(server_url: Option<String>) -> Result<Self> {
        let mut config = config::Config::builder()
            .set_default("server.url", None::<Option<String>>)?
            .set_default("playback.speed", None::<Option<f64>>)?
            .set_default("session.capture_input", false)?
            .set_default("notifications.enabled", true)?
            .add_source(File::with_name("/etc/asciinema/config.toml").required(false))
            .add_source(File::with_name(&user_defaults_path()?.to_string_lossy()).required(false))
            .add_source(File::with_name(&user_config_path()?.to_string_lossy()).required(false));

        // legacy env var
        if let Ok(url) = env::var("ASCIINEMA_API_URL") {
            config = config.set_override("server.url", Some(url))?;
        }

        if let Ok(url) = env::var("ASCIINEMA_SERVER_URL") {
            config = config.set_override("server.url", Some(url))?;
        }

        if let Some(url) = server_url {
            config = config.set_override("server.url", Some(url))?;
        }

        Ok(config.build()?.try_deserialize()?)
    }

    pub fn get_server_url(&mut self) -> Result<Url> {
        match self.server.url.as_ref() {
            Some(url) => Ok(parse_server_url(url)?),

            None => {
                let url = parse_server_url(&ask_for_server_url()?)?;
                save_default_server_url(url.as_ref())?;
                self.server.url = Some(url.to_string());

                Ok(url)
            }
        }
    }

    pub fn get_install_id(&self) -> Result<String> {
        let path = install_id_path()?;
        let legacy_path = legacy_install_id_path()?;

        if let Some(id) = read_install_id(&path)? {
            Ok(id)
        } else if let Some(id) = read_install_id(&legacy_path)? {
            Ok(id)
        } else {
            let id = generate_install_id();
            save_install_id(&path, &id)?;

            Ok(id)
        }
    }
}

impl Session {
    pub fn prefix_key(&self) -> Result<Option<Key>> {
        self.prefix_key.as_ref().map(parse_key).transpose()
    }

    pub fn pause_key(&self) -> Result<Option<Key>> {
        self.pause_key.as_ref().map(parse_key).transpose()
    }

    pub fn add_marker_key(&self) -> Result<Option<Key>> {
        self.add_marker_key.as_ref().map(parse_key).transpose()
    }
}

impl Playback {
    pub fn pause_key(&self) -> Result<Option<Key>> {
        self.pause_key.as_ref().map(parse_key).transpose()
    }

    pub fn step_key(&self) -> Result<Option<Key>> {
        self.step_key.as_ref().map(parse_key).transpose()
    }

    pub fn next_marker_key(&self) -> Result<Option<Key>> {
        self.next_marker_key.as_ref().map(parse_key).transpose()
    }
}

fn ask_for_server_url() -> Result<String> {
    println!("No asciinema server configured for this CLI.");

    let url = rustyline::DefaultEditor::new()?.readline_with_initial(
        "Enter the server URL to use by default: ",
        (DEFAULT_SERVER_URL, ""),
    )?;

    println!();

    Ok(url)
}

fn save_default_server_url(url: &str) -> Result<()> {
    let path = user_defaults_path()?;

    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }

    fs::write(path, format!("[server]\nurl = \"{url}\"\n"))?;

    Ok(())
}

fn parse_server_url(s: &str) -> Result<Url> {
    let url = Url::parse(s)?;

    if url.host().is_none() {
        bail!("server URL is missing a host");
    }

    Ok(url)
}

fn read_install_id(path: &PathBuf) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(s) => Ok(Some(s.trim().to_string())),

        Err(e) => {
            if e.kind() == ErrorKind::NotFound {
                Ok(None)
            } else {
                bail!(e)
            }
        }
    }
}

fn generate_install_id() -> String {
    Uuid::new_v4().to_string()
}

fn save_install_id(path: &PathBuf, id: &str) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }

    fs::write(path, id)?;

    Ok(())
}

pub fn user_config_path() -> Result<PathBuf> {
    Ok(config_home()?.join("config.toml"))
}

fn legacy_user_config_path() -> Result<PathBuf> {
    Ok(config_home()?.join("config"))
}

fn user_defaults_path() -> Result<PathBuf> {
    Ok(config_home()?.join("defaults.toml"))
}

fn install_id_path() -> Result<PathBuf> {
    Ok(state_home()?.join(INSTALL_ID_FILENAME))
}

fn legacy_install_id_path() -> Result<PathBuf> {
    Ok(config_home()?.join(INSTALL_ID_FILENAME))
}

fn config_home() -> Result<PathBuf> {
    env::var("ASCIINEMA_CONFIG_HOME")
        .map(PathBuf::from)
        .or(env::var("XDG_CONFIG_HOME").map(|home| Path::new(&home).join("asciinema")))
        .or(env::var("HOME").map(|home| Path::new(&home).join(".config").join("asciinema")))
        .map_err(|_| anyhow!("need $HOME or $XDG_CONFIG_HOME or $ASCIINEMA_CONFIG_HOME"))
}

fn state_home() -> Result<PathBuf> {
    env::var("ASCIINEMA_STATE_HOME")
        .map(PathBuf::from)
        .or(env::var("XDG_STATE_HOME").map(|home| Path::new(&home).join("asciinema")))
        .or(env::var("HOME").map(|home| {
            Path::new(&home)
                .join(".local")
                .join("state")
                .join("asciinema")
        }))
        .map_err(|_| anyhow!("need $HOME or $XDG_STATE_HOME or $ASCIINEMA_STATE_HOME"))
}

fn parse_key<S: AsRef<str>>(key: S) -> Result<Key> {
    let key = key.as_ref();
    let chars: Vec<char> = key.chars().collect();

    match chars.len() {
        0 => return Ok(None),

        1 => {
            let mut buf = [0; 4];
            let str = chars[0].encode_utf8(&mut buf);

            return Ok(Some(str.as_bytes().into()));
        }

        2 => {
            if chars[0] == '^' && chars[1].is_ascii_alphabetic() {
                let key = vec![chars[1].to_ascii_uppercase() as u8 - 0x40];

                return Ok(Some(key));
            }
        }

        3 => {
            if chars[0].eq_ignore_ascii_case(&'C')
                && ['+', '-'].contains(&chars[1])
                && chars[2].is_ascii_alphabetic()
            {
                let key = vec![chars[2].to_ascii_uppercase() as u8 - 0x40];

                return Ok(Some(key));
            }
        }

        _ => (),
    }

    Err(anyhow!("invalid key definition '{key}'"))
}

pub fn check_legacy_config_file() {
    let Ok(legacy_path) = legacy_user_config_path() else {
        return;
    };

    let Ok(new_path) = user_config_path() else {
        return;
    };

    if legacy_path.exists() && !new_path.exists() {
        status::warning!(
            "Your config file at {} uses the location and format from asciinema 2.x.",
            legacy_path.to_string_lossy()
        );

        status::warning!(
            "For asciinema 3.x (this version) create a new config file at {}.",
            new_path.to_string_lossy()
        );

        status::warning!("Read the documentation (CLI -> Configuration) for details.\n");
    }
}
