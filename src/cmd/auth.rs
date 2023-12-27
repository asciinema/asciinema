use crate::{config::Config, util};
use anyhow::{anyhow, Result};
use clap::Args;
use reqwest::Url;

#[derive(Debug, Args)]
pub struct Cli {}

impl Cli {
    pub fn run(self, config: &Config) -> Result<()> {
        let auth_url = auth_url(config.server_url())?;
        let server_hostname = auth_url.host().ok_or(anyhow!("invalid server URL"))?;

        println!("Open the following URL in a web browser to authenticate this asciinema CLI with your {server_hostname} user account:\n");
        println!("{}\n", auth_url);
        println!("This action will associate all recordings uploaded from this machine (past and future ones) with your account, allowing you to manage them (change the title/theme, delete) at {server_hostname}.");

        Ok(())
    }
}

fn auth_url(server_url: Option<&String>) -> Result<Url> {
    let mut url = util::get_server_url(server_url)?;
    url.set_path(&format!("connect/{}", util::get_install_id()?));

    Ok(url)
}
