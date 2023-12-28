use crate::config::Config;
use anyhow::{anyhow, Result};
use clap::Args;
use reqwest::Url;

#[derive(Debug, Args)]
pub struct Cli {}

impl Cli {
    pub fn run(self, config: &Config) -> Result<()> {
        let server_url = config.get_server_url()?;
        let server_hostname = server_url.host().ok_or(anyhow!("invalid server URL"))?;
        let auth_url = auth_url(&server_url, &config.get_install_id()?);

        println!("Open the following URL in a web browser to authenticate this asciinema CLI with your {server_hostname} user account:\n");
        println!("{}\n", auth_url);
        println!("This action will associate all recordings uploaded from this machine (past and future ones) with your account, allowing you to manage them (change the title/theme, delete) at {server_hostname}.");

        Ok(())
    }
}

fn auth_url(server_url: &Url, install_id: &str) -> Url {
    let mut url = server_url.clone();
    url.set_path(&format!("connect/{install_id}"));

    url
}
