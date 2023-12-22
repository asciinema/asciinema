use crate::util;
use anyhow::{anyhow, Result};
use reqwest::Url;

pub fn run(server_url: String) -> Result<()> {
    let mut auth_url = Url::parse(&server_url)?;
    let install_id = util::get_install_id()?;
    auth_url.set_path(&format!("connect/{install_id}"));
    let server_hostname = auth_url.host().ok_or(anyhow!("invalid server URL"))?;

    println!("Open the following URL in a web browser to authenticate this asciinema CLI with your {server_hostname} user account:\n");
    println!("{}\n", auth_url);
    println!("This action will associate all recordings uploaded from this machine (past and future ones) with your account, allowing you to manage them (change the title/theme, delete) at {server_hostname}.");

    Ok(())
}
