use crate::config::Config;
use crate::format::asciicast;
use anyhow::{anyhow, Result};
use clap::Args;
use reqwest::{
    blocking::{multipart::Form, Client},
    header, Url,
};
use serde::Deserialize;
use std::env;

#[derive(Debug, Args)]
pub struct Cli {
    /// Filename/path of asciicast to upload
    filename: String,
}

#[derive(Debug, Deserialize)]
struct UploadResponse {
    url: String,
    message: Option<String>,
}

impl Cli {
    pub fn run(self, config: &Config) -> Result<()> {
        let _ = asciicast::open_from_path(&self.filename)?;
        let client = Client::new();
        let form = Form::new().file("asciicast", self.filename)?;

        let response = client
            .post(api_url(&config.get_server_url()?))
            .multipart(form)
            .basic_auth(get_username(), Some(config.get_install_id()?))
            .header(header::USER_AGENT, build_user_agent())
            .header(header::ACCEPT, "application/json")
            .send()?;

        response.error_for_status_ref()?;

        let content_type = response
            .headers()
            .get("content-type")
            .ok_or(anyhow!("no content-type header in the response"))?
            .to_str()?;

        if content_type.starts_with("application/json") {
            let json = response.json::<UploadResponse>()?;

            if let Some(message) = json.message {
                println!("{}", message);
            } else {
                println!("{}", json.url);
            }
        } else {
            println!("{}", &response.text()?);
        }

        Ok(())
    }
}

fn api_url(server_url: &Url) -> Url {
    let mut url = server_url.clone();
    url.set_path("api/asciicasts");

    url
}

fn get_username() -> String {
    env::var("USER").unwrap_or("".to_owned())
}

fn build_user_agent() -> String {
    let ua = concat!("asciinema/", env!("CARGO_PKG_VERSION")); // TODO add more system info

    ua.to_owned()
}
