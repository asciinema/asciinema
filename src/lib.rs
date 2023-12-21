mod util;
use anyhow::{anyhow, Result};
use reqwest::{
    blocking::{multipart::Form, Client},
    header, Url,
};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize)]
struct UploadResponse {
    url: String,
    message: Option<String>,
}

pub fn upload(filename: String, server_url: String) -> Result<()> {
    let mut api_url = Url::parse(&server_url)?;
    api_url.set_path("api/asciicasts");
    let install_id = util::get_install_id()?;
    let client = Client::new();
    let form = Form::new().file("asciicast", filename)?;

    let response = client
        .post(api_url)
        .multipart(form)
        .basic_auth(get_username(), Some(install_id))
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

fn get_username() -> String {
    env::var("USER").unwrap_or("".to_owned())
}

fn build_user_agent() -> String {
    let ua = concat!("asciinema/", env!("CARGO_PKG_VERSION")); // TODO add more system info

    ua.to_owned()
}

pub fn auth(server_url: String) -> Result<()> {
    let mut auth_url = Url::parse(&server_url)?;
    let install_id = util::get_install_id()?;
    auth_url.set_path(&format!("connect/{install_id}"));
    let server_hostname = auth_url.host().ok_or(anyhow!("invalid server URL"))?;

    println!("Open the following URL in a web browser to authenticate this asciinema CLI with your {server_hostname} user account:\n");
    println!("{}\n", auth_url);
    println!("This action will associate all recordings uploaded from this machine (past and future ones) with your account, allowing you to manage them (change the title/theme, delete) at {server_hostname}.");

    Ok(())
}
