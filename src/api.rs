use std::env;
use std::fmt::Debug;

use anyhow::{bail, Context, Result};
use reqwest::{header, Response};
use reqwest::{multipart::Form, Client, RequestBuilder};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::config::Config;

#[derive(Debug, Deserialize)]
pub struct RecordingResponse {
    pub url: String,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StreamResponse {
    pub id: u64,
    pub ws_producer_url: String,
    pub url: String,
}

#[derive(Default, Serialize)]
pub struct StreamChangeset {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<Option<u8>>,
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

pub async fn create_recording(path: &str, config: &Config) -> Result<RecordingResponse> {
    let server_url = &config.get_server_url()?;
    let install_id = config.get_install_id()?;

    let response = create_recording_request(server_url, path, install_id)
        .await?
        .send()
        .await?;

    if response.status().as_u16() == 413 {
        bail!("The size of the recording exceeds the server's configured limit");
    }

    response.error_for_status_ref()?;

    Ok(response.json::<RecordingResponse>().await?)
}

async fn create_recording_request(
    server_url: &Url,
    path: &str,
    install_id: String,
) -> Result<RequestBuilder> {
    let client = Client::new();
    let mut url = server_url.clone();
    url.set_path("api/v1/recordings");
    let form = Form::new().file("file", path).await?;

    Ok(client
        .post(url)
        .multipart(form)
        .basic_auth(get_username(), Some(install_id))
        .header(header::USER_AGENT, build_user_agent())
        .header(header::ACCEPT, "application/json"))
}

pub async fn list_user_streams(prefix: &str, config: &Config) -> Result<Vec<StreamResponse>> {
    let server_url = config.get_server_url()?;
    let install_id = config.get_install_id()?;

    let response = list_user_streams_request(&server_url, prefix, &install_id)
        .send()
        .await
        .context("cannot obtain stream producer endpoint - is the server down?")?;

    parse_stream_response(response, &server_url).await
}

fn list_user_streams_request(server_url: &Url, prefix: &str, install_id: &str) -> RequestBuilder {
    let client = Client::new();
    let mut url = server_url.clone();
    url.set_path("api/v1/user/streams");
    url.set_query(Some(&format!("prefix={prefix}&limit=10")));

    add_headers(client.get(url), install_id)
}

pub async fn create_stream(changeset: StreamChangeset, config: &Config) -> Result<StreamResponse> {
    let server_url = config.get_server_url()?;
    let install_id = config.get_install_id()?;

    let response = create_stream_request(&server_url, &install_id, changeset)
        .send()
        .await
        .context("cannot obtain stream producer endpoint - is the server down?")?;

    parse_stream_response(response, &server_url).await
}

fn create_stream_request(
    server_url: &Url,
    install_id: &str,
    changeset: StreamChangeset,
) -> RequestBuilder {
    let client = Client::new();
    let mut url = server_url.clone();
    url.set_path("api/v1/streams");
    let builder = client.post(url);
    let builder = add_headers(builder, install_id);

    builder.json(&changeset)
}

pub async fn update_stream(
    stream_id: u64,
    changeset: StreamChangeset,
    config: &Config,
) -> Result<StreamResponse> {
    let server_url = config.get_server_url()?;
    let install_id = config.get_install_id()?;

    let response = update_stream_request(&server_url, &install_id, stream_id, changeset)
        .send()
        .await
        .context("cannot obtain stream producer endpoint - is the server down?")?;

    parse_stream_response(response, &server_url).await
}

fn update_stream_request(
    server_url: &Url,
    install_id: &str,
    stream_id: u64,
    changeset: StreamChangeset,
) -> RequestBuilder {
    let client = Client::new();
    let mut url = server_url.clone();
    url.set_path(&format!("api/v1/streams/{stream_id}"));
    let builder = client.patch(url);
    let builder = add_headers(builder, install_id);

    builder.json(&changeset)
}

async fn parse_stream_response<T: DeserializeOwned>(
    response: Response,
    server_url: &Url,
) -> Result<T> {
    let server_hostname = server_url.host().unwrap();

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

    response.json::<T>().await.map_err(|e| e.into())
}

fn add_headers(builder: RequestBuilder, install_id: &str) -> RequestBuilder {
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
