use anyhow::Result;

use crate::api;
use crate::cli;
use crate::config::Config;

impl cli::Auth {
    pub fn run(self) -> Result<()> {
        let config = Config::new(self.server_url.clone())?;
        let server_url = config.get_server_url()?;
        let server_hostname = server_url.host().unwrap();
        let auth_url = api::get_auth_url(&config)?;

        println!("Open the following URL in a web browser to authenticate this asciinema CLI with your {server_hostname} user account:\n");
        println!("{auth_url}\n");
        println!("This action will associate all recordings uploaded from this machine (past and future ones) with your account, allowing you to manage them (change the title/theme, delete) at {server_hostname}.");

        Ok(())
    }
}
