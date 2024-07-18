use crate::config::Config;
use anyhow::{bail, Context, Result};
use reqwest::blocking::ClientBuilder;
use reqwest::blocking::{multipart::Form, Client, RequestBuilder};
use reqwest::{header, Certificate};
use serde::Deserialize;
use std::env;
use std::fmt::Debug;
use std::fs::read_to_string;
use url::Url;

#[derive(Debug, Deserialize)]
pub struct UploadAsciicastResponse {
    pub url: String,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GetUserStreamResponse {
    pub ws_producer_url: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
struct NotFoundResponse {
    reason: String,
}

pub fn get_auth_url(config: &Config) -> Result<Url> {
    let mut url = config.get_server_url()?;
    url.set_path(&format!("connect/{}", config.get_install_id()?));

    Ok(url)
}

pub fn upload_asciicast(path: &str, config: &Config) -> Result<UploadAsciicastResponse> {
    let server_url = &config.get_server_url()?;
    let install_id = config.get_install_id()?;
    let response = upload_request(server_url, path, install_id)?.send()?;

    if response.status().as_u16() == 413 {
        bail!("The size of the recording exceeds the server's configured limit");
    }

    response.error_for_status_ref()?;

    Ok(response.json::<UploadAsciicastResponse>()?)
}

fn upload_request(server_url: &Url, path: &str, install_id: String) -> Result<RequestBuilder> {
    let client = if let Ok(ca) = env::var("REQUESTS_CA_BUNDLE") {
        let ca = Certificate::from_pem(read_to_string(ca)?.as_bytes())?;
        ClientBuilder::new().add_root_certificate(ca).build()?
    } else {
        Client::new()
    };

    let mut url = server_url.clone();
    url.set_path("api/asciicasts");
    let form = Form::new().file("asciicast", path)?;

    Ok(client
        .post(url)
        .multipart(form)
        .basic_auth(get_username(), Some(install_id))
        .header(header::USER_AGENT, build_user_agent())
        .header(header::ACCEPT, "application/json"))
}

pub fn create_user_stream(stream_id: String, config: &Config) -> Result<GetUserStreamResponse> {
    let server_url = config.get_server_url()?;
    let server_hostname = server_url.host().unwrap();
    let install_id = config.get_install_id()?;

    let response = user_stream_request(&server_url, stream_id, install_id)
        .send()
        .context("cannot obtain stream producer endpoint")?;

    match response.status().as_u16() {
        401 => bail!(
            "this CLI hasn't been authenticated with {server_hostname} - run `ascinema auth` first"
        ),

        404 => match response.json::<NotFoundResponse>() {
            Ok(json) => bail!("{}", json.reason),
            Err(_) => bail!("{server_hostname} doesn't support streaming"),
        },

        _ => {
            response.error_for_status_ref()?;
        }
    }

    response
        .json::<GetUserStreamResponse>()
        .map_err(|e| e.into())
}

fn user_stream_request(server_url: &Url, stream_id: String, install_id: String) -> RequestBuilder {
    let client = Client::new();
    let mut url = server_url.clone();

    let builder = if stream_id.is_empty() {
        url.set_path("api/streams");
        client.post(url)
    } else {
        url.set_path(&format!("api/user/streams/{stream_id}"));
        client.get(url)
    };

    builder
        .basic_auth(get_username(), Some(install_id))
        .header(header::USER_AGENT, build_user_agent())
        .header(header::ACCEPT, "application/json")
}

fn get_username() -> String {
    env::var("USER").unwrap_or("".to_owned())
}

fn build_user_agent() -> String {
    let ua = concat!(
        "asciinema/",
        env!("CARGO_PKG_VERSION"),
        " target/",
        env!("TARGET")
    );

    ua.to_owned()
}
