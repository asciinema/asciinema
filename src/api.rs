use std::env;
use std::fmt::Debug;

use anyhow::{bail, Context, Result};
use reqwest::header;
use reqwest::{multipart::Form, Client, RequestBuilder};
use serde::Deserialize;
use url::Url;

use crate::config::Config;

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
struct ErrorResponse {
    reason: String,
}

pub fn get_auth_url(config: &Config) -> Result<Url> {
    let mut url = config.get_server_url()?;
    url.set_path(&format!("connect/{}", config.get_install_id()?));

    Ok(url)
}

pub async fn upload_asciicast(path: &str, config: &Config) -> Result<UploadAsciicastResponse> {
    let server_url = &config.get_server_url()?;
    let install_id = config.get_install_id()?;

    let response = upload_request(server_url, path, install_id)
        .await?
        .send()
        .await?;

    if response.status().as_u16() == 413 {
        bail!("The size of the recording exceeds the server's configured limit");
    }

    response.error_for_status_ref()?;

    Ok(response.json::<UploadAsciicastResponse>().await?)
}

async fn upload_request(
    server_url: &Url,
    path: &str,
    install_id: String,
) -> Result<RequestBuilder> {
    let client = Client::new();
    let mut url = server_url.clone();
    url.set_path("api/asciicasts");
    let form = Form::new().file("asciicast", path).await?;

    Ok(client
        .post(url)
        .multipart(form)
        .basic_auth(get_username(), Some(install_id))
        .header(header::USER_AGENT, build_user_agent())
        .header(header::ACCEPT, "application/json"))
}

pub async fn create_user_stream(stream_id: &str, config: &Config) -> Result<GetUserStreamResponse> {
    let server_url = config.get_server_url()?;
    let server_hostname = server_url.host().unwrap();
    let install_id = config.get_install_id()?;

    let response = user_stream_request(&server_url, stream_id, &install_id)
        .send()
        .await
        .context("cannot obtain stream producer endpoint - is the server down?")?;

    match response.status().as_u16() {
        401 => bail!(
            "this CLI hasn't been authenticated with {server_hostname} - run `asciinema auth` first"
        ),

        404 => match response.json::<ErrorResponse>().await {
            Ok(json) => bail!("{}", json.reason),
            Err(_) => bail!("{server_hostname} doesn't support streaming"),
        },

        422 => match response.json::<ErrorResponse>().await {
            Ok(json) => bail!("{}", json.reason),
            Err(_) => bail!("{server_hostname} doesn't support streaming"),
        },

        _ => {
            response.error_for_status_ref()?;
        }
    }

    response
        .json::<GetUserStreamResponse>()
        .await
        .map_err(|e| e.into())
}

fn user_stream_request(server_url: &Url, stream_id: &str, install_id: &str) -> RequestBuilder {
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

pub fn build_user_agent() -> String {
    let ua = concat!(
        "asciinema/",
        env!("CARGO_PKG_VERSION"),
        " target/",
        env!("TARGET")
    );

    ua.to_owned()
}
