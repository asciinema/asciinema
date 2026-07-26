use std::collections::HashMap;
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

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    Public,
    Unlisted,
    Private,
}

#[derive(Default, Serialize)]
pub struct RecordingChangeset {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<Visibility>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_url: Option<Option<String>>,
}

#[derive(Default, Serialize)]
pub struct StreamChangeset {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<Visibility>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_url: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub term_type: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub term_version: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<Option<HashMap<String, String>>>,
}

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    #[serde(rename = "type")]
    error_type: Option<String>,
    message: Option<String>,
    details: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct ErrorDetail {
    field: Option<String>,
    message: String,
}

pub fn get_auth_url(config: &mut Config) -> Result<Url> {
    let mut url = config.get_server_url()?;
    url.set_path(&format!("connect/{}", config.get_install_id()?));

    Ok(url)
}

pub async fn create_recording(
    path: &str,
    changeset: RecordingChangeset,
    config: &mut Config,
) -> Result<RecordingResponse> {
    let server_url = &config.get_server_url()?;
    let install_id = config.get_install_id()?;

    let response = create_recording_request(server_url, install_id, path, changeset)
        .await?
        .send()
        .await?;

    let legacy_fallback = (response.status().as_u16() == 413)
        .then(|| "The recording exceeds the server-configured size limit".to_owned());

    let server_hostname = server_url
        .host()
        .expect("host presence is checked in parse_server_url")
        .to_string();
    let response = handle_response_status(response, &server_hostname, legacy_fallback).await?;

    Ok(response.json::<RecordingResponse>().await?)
}

async fn create_recording_request(
    server_url: &Url,
    install_id: String,
    path: &str,
    changeset: RecordingChangeset,
) -> Result<RequestBuilder> {
    let client = Client::new();
    let mut url = server_url.clone();
    url.set_path("api/v1/recordings");
    let form = Form::new().file("file", path).await?;
    let form = add_recording_changeset_fields(form, changeset);
    let builder = client.post(url).multipart(form);

    Ok(add_headers(builder, &install_id))
}

fn add_recording_changeset_fields(mut form: Form, changeset: RecordingChangeset) -> Form {
    if let Some(Some(title)) = changeset.title {
        form = form.text("title", title);
    }

    if let Some(Some(description)) = changeset.description {
        form = form.text("description", description);
    }

    if let Some(visibility) = changeset.visibility {
        let visibility = match visibility {
            Visibility::Public => "public",
            Visibility::Unlisted => "unlisted",
            Visibility::Private => "private",
        };

        form = form.text("visibility", visibility);
    }

    if let Some(Some(audio_url)) = changeset.audio_url {
        form = form.text("audio_url", audio_url);
    }

    form
}

pub async fn list_user_streams(prefix: &str, config: &mut Config) -> Result<Vec<StreamResponse>> {
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

pub async fn create_stream(
    changeset: StreamChangeset,
    config: &mut Config,
) -> Result<StreamResponse> {
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
    config: &mut Config,
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
    let server_hostname = server_url
        .host()
        .expect("host presence is checked in parse_server_url")
        .to_string();

    let legacy_fallback = match response.status().as_u16() {
        404 | 422 => Some(format!("{server_hostname} doesn't support streaming")),
        _ => None,
    };

    let response = handle_response_status(response, &server_hostname, legacy_fallback).await?;

    response.json::<T>().await.map_err(|e| e.into())
}

async fn handle_response_status(
    response: Response,
    server_hostname: &str,
    legacy_fallback: Option<String>,
) -> Result<Response> {
    let status_error = match response.error_for_status_ref() {
        Ok(_) => return Ok(response),
        Err(error) => error,
    };

    let message = match response.bytes().await {
        Ok(body) => parse_error_response(&body)
            .and_then(|response| render_error_response(response, server_hostname)),
        Err(_) => None,
    };

    if let Some(message) = message.or(legacy_fallback) {
        bail!(message);
    }

    Err(status_error.into())
}

fn parse_error_response(body: &[u8]) -> Option<ErrorResponse> {
    serde_json::from_slice(body).ok()
}

fn render_error_response(response: ErrorResponse, server_hostname: &str) -> Option<String> {
    let guidance = match response.error_type.as_deref() {
        Some("account_required") => Some(format!(
            "Run `asciinema auth` to link this CLI to your {server_hostname} account."
        )),

        Some("upload_limit_reached") => {
            Some("Run `asciinema auth` and follow the link to upload more.".to_owned())
        }

        _ => None,
    };

    let mut message = format_error_response(response)?;

    if let Some(guidance) = guidance {
        message.push_str("\n\n");
        message.push_str(&guidance);
    }

    Some(message)
}

fn format_error_response(response: ErrorResponse) -> Option<String> {
    let mut message = response
        .message
        .filter(|message| !message.trim().is_empty())?;

    if response.error_type.as_deref() != Some("validation_failed") {
        return Some(message);
    }

    if let Some(serde_json::Value::Array(details)) = response.details {
        let mut has_details = false;

        for value in details {
            let Ok(detail) = serde_json::from_value::<ErrorDetail>(value) else {
                continue;
            };

            if detail.message.trim().is_empty() {
                continue;
            }

            if !has_details {
                if !message.ends_with(':') {
                    message.push(':');
                }

                has_details = true;
            }

            match detail.field.as_deref().map(str::trim) {
                Some(field) if !field.is_empty() && field != "." => {
                    message.push_str(&format!("\n  {field}: {}", detail.message));
                }
                _ => message.push_str(&format!("\n  {}", detail.message)),
            }
        }
    }

    Some(message)
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

#[cfg(test)]
mod tests {
    use axum::http::{Response as HttpResponse, StatusCode};
    use tokio::runtime::Runtime;

    use super::{
        format_error_response, handle_response_status, parse_error_response, render_error_response,
    };

    const SERVER_HOSTNAME: &str = "example.com";

    fn response(status: StatusCode, body: &'static str) -> reqwest::Response {
        HttpResponse::builder()
            .status(status)
            .body(body)
            .unwrap()
            .into()
    }

    fn parse_error_message(body: &[u8]) -> Option<String> {
        parse_error_response(body).and_then(format_error_response)
    }

    fn render_error_message(body: &[u8]) -> Option<String> {
        parse_error_response(body)
            .and_then(|response| render_error_response(response, SERVER_HOSTNAME))
    }

    #[test]
    fn augments_account_required_error() {
        let body = br#"{
            "type": "account_required",
            "message": "This action requires an account"
        }"#;

        assert_eq!(
            render_error_message(body),
            Some(
                "This action requires an account\n\n\
                 Run `asciinema auth` to link this CLI to your example.com account."
                    .to_owned()
            )
        );
    }

    #[test]
    fn augments_upload_limit_reached_error() {
        let body = br#"{
            "type": "upload_limit_reached",
            "message": "Anonymous upload limit reached"
        }"#;

        assert_eq!(
            render_error_message(body),
            Some(
                "Anonymous upload limit reached\n\n\
                 Run `asciinema auth` and follow the link to upload more."
                    .to_owned()
            )
        );

        let error = Runtime::new()
            .unwrap()
            .block_on(handle_response_status(
                response(StatusCode::FORBIDDEN, std::str::from_utf8(body).unwrap()),
                SERVER_HOSTNAME,
                None,
            ))
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "Anonymous upload limit reached\n\n\
             Run `asciinema auth` and follow the link to upload more."
        );
    }

    #[test]
    fn leaves_unauthenticated_error_verbatim() {
        let body = br#"{
            "type": "unauthenticated",
            "message": "Installation ID has been revoked"
        }"#;

        assert_eq!(
            render_error_message(body),
            Some("Installation ID has been revoked".to_owned())
        );
    }

    #[test]
    fn leaves_unknown_error_type_verbatim() {
        let body = br#"{
            "type": "future_error",
            "message": "A future server error"
        }"#;

        assert_eq!(
            render_error_message(body),
            Some("A future server error".to_owned())
        );
    }

    #[test]
    fn formats_validation_error_details() {
        let body = br#"{
            "type": "validation_failed",
            "message": "Validation failed",
            "details": [
                {
                    "field": "audio_url",
                    "message": "has invalid format"
                },
                {
                    "field": "audio_url",
                    "message": "should be at most 255 character(s)"
                }
            ]
        }"#;

        assert_eq!(
            render_error_message(body),
            Some(
                "Validation failed:\n  audio_url: has invalid format\n  \
                 audio_url: should be at most 255 character(s)"
                    .to_owned()
            )
        );
    }

    #[test]
    fn ignores_invalid_validation_error_details() {
        for body in [
            br#"{
                "type": "validation_failed",
                "message": "Validation failed"
            }"#
            .as_slice(),
            br#"{
                "type": "validation_failed",
                "message": "Validation failed",
                "details": "invalid"
            }"#
            .as_slice(),
            br#"{
                "type": "validation_failed",
                "message": "Validation failed",
                "details": [
                    {"field": "idle_time_limit"},
                    {"field": "", "message": ""}
                ]
            }"#
            .as_slice(),
        ] {
            assert_eq!(
                parse_error_message(body),
                Some("Validation failed".to_owned())
            );
        }
    }

    #[test]
    fn formats_fieldless_validation_error_details() {
        let body = br#"{
            "type": "validation_failed",
            "message": "Validation failed",
            "details": [
                {"message": "recording metadata is invalid"},
                {"field": "", "message": "recording data is invalid"},
                {"field": "  ", "message": "recording options are invalid"},
                {"field": ".", "message": "recording is invalid"}
            ]
        }"#;

        assert_eq!(
            parse_error_message(body),
            Some(
                "Validation failed:\n  recording metadata is invalid\n  \
                 recording data is invalid\n  recording options are invalid\n  recording is invalid"
                    .to_owned()
            )
        );
    }

    #[test]
    fn ignores_error_body_without_message() {
        assert_eq!(
            parse_error_message(br#"{"type":"upload_limit_reached"}"#),
            None
        );
        assert_eq!(parse_error_message(br#"{"message":""}"#), None);
    }

    #[test]
    fn falls_back_to_status_error_for_invalid_or_empty_body() {
        assert_eq!(parse_error_message(b"not JSON"), None);
        assert_eq!(parse_error_message(b""), None);

        for body in ["not JSON", ""] {
            let error = Runtime::new()
                .unwrap()
                .block_on(handle_response_status(
                    response(StatusCode::UNPROCESSABLE_ENTITY, body),
                    SERVER_HOSTNAME,
                    Some("The server doesn't support streaming".to_owned()),
                ))
                .unwrap_err();

            assert_eq!(error.to_string(), "The server doesn't support streaming");
        }

        for body in ["not JSON", ""] {
            let error = Runtime::new()
                .unwrap()
                .block_on(handle_response_status(
                    response(StatusCode::FORBIDDEN, body),
                    SERVER_HOSTNAME,
                    None,
                ))
                .unwrap_err();

            assert!(error.to_string().contains("403 Forbidden"));
        }
    }
}
