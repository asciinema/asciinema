use crate::config;
use anyhow::{anyhow, bail, Result};
use reqwest::Url;
use std::{env, fs, io::ErrorKind, path::Path, path::PathBuf};
use uuid::Uuid;

const DEFAULT_SERVER_URL: &str = "https://asciinema.org";
const INSTALL_ID_FILENAME: &str = "install-id";

pub fn get_install_id() -> Result<String> {
    if let Some(install_id) = read_install_id()? {
        Ok(install_id)
    } else if let Some(install_id) = read_legacy_install_id()? {
        Ok(install_id)
    } else {
        create_install_id()
    }
}

fn read_install_id() -> Result<Option<String>> {
    read_state_file(INSTALL_ID_FILENAME)
}

fn create_install_id() -> Result<String> {
    let id = Uuid::new_v4().to_string();
    write_state_file(INSTALL_ID_FILENAME, &id)?;

    Ok(id)
}

fn read_legacy_install_id() -> Result<Option<String>> {
    let path = config::home()?.join(INSTALL_ID_FILENAME);

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

pub fn get_server_url(server_url: Option<&String>) -> Result<Url> {
    match server_url {
        Some(url) => Ok(Url::parse(&url)?),

        None => {
            let url = Url::parse(&ask_for_server_url()?)?;
            config::save_default_server_url(url.as_ref())?;

            Ok(url)
        }
    }
}

fn ask_for_server_url() -> Result<String> {
    println!("No asciinema server configured for this CLI.");
    let mut rl = rustyline::DefaultEditor::new()?;
    let url = rl.readline_with_initial(
        "Enter the server URL to use by default: ",
        (DEFAULT_SERVER_URL, ""),
    )?;
    println!();

    Ok(url)
}

fn read_state_file(filename: &str) -> Result<Option<String>> {
    let path = state_home()?.join(filename);

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

fn write_state_file(filename: &str, contents: &str) -> Result<()> {
    let path = state_home()?.join(filename);

    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }

    fs::write(path, contents)?;

    Ok(())
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
