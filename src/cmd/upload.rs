use crate::util;
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

pub fn run(filename: String, server_url: String) -> Result<()> {
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
