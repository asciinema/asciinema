use anyhow::{anyhow, bail, Result};
use std::{env, fs, io::ErrorKind, path::Path, path::PathBuf};
use uuid::Uuid;

pub fn get_install_id() -> Result<String> {
    let default_path = state_home()?.join("install-id");
    let legacy_path = config_home()?.join("install-id");

    if let Some(install_id) = read_install_id(&default_path)? {
        Ok(install_id)
    } else if let Some(install_id) = read_install_id(&legacy_path)? {
        Ok(install_id)
    } else {
        create_install_id(&default_path)
    }
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

fn create_install_id(path: &PathBuf) -> Result<String> {
    let id = Uuid::new_v4().to_string();

    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }

    fs::write(path, &id)?;

    Ok(id)
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
