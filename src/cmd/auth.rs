use crate::util;
use anyhow::{anyhow, Result};
use clap::Args;
use reqwest::Url;

#[derive(Debug, Args)]
pub struct Cli {
    /// asciinema server URL
    server_url: String,
}

impl Cli {
    pub fn run(self) -> Result<()> {
        let auth_url = self.auth_url()?;
        let server_hostname = auth_url.host().ok_or(anyhow!("invalid server URL"))?;

        println!("Open the following URL in a web browser to authenticate this asciinema CLI with your {server_hostname} user account:\n");
        println!("{}\n", auth_url);
        println!("This action will associate all recordings uploaded from this machine (past and future ones) with your account, allowing you to manage them (change the title/theme, delete) at {server_hostname}.");

        Ok(())
    }

    fn auth_url(&self) -> Result<Url> {
        let mut url = Url::parse(&self.server_url)?;
        url.set_path(&format!("connect/{}", util::get_install_id()?));

        Ok(url)
    }
}
