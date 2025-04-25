use std::collections::HashMap;

use anyhow::{bail, Result};
use serde::Deserialize;

use super::{Asciicast, Event, Header};
use crate::asciicast::util::deserialize_time;

#[derive(Debug, Deserialize)]
struct V1 {
    version: u8,
    width: u16,
    height: u16,
    command: Option<String>,
    title: Option<String>,
    env: Option<HashMap<String, String>>,
    stdout: Vec<V1OutputEvent>,
}

#[derive(Debug, Deserialize)]
struct V1OutputEvent {
    #[serde(deserialize_with = "deserialize_time")]
    time: u64,
    data: String,
}

pub fn load(json: String) -> Result<Asciicast<'static>> {
    let asciicast: V1 = serde_json::from_str(&json)?;

    if asciicast.version != 1 {
        bail!("unsupported asciicast version")
    }

    let term_type = asciicast
        .env
        .as_ref()
        .and_then(|env| env.get("TERM"))
        .cloned();

    let header = Header {
        term_cols: asciicast.width,
        term_rows: asciicast.height,
        term_type,
        term_version: None,
        term_theme: None,
        timestamp: None,
        idle_time_limit: None,
        command: asciicast.command.clone(),
        title: asciicast.title.clone(),
        env: asciicast.env.clone(),
    };

    let events = Box::new(
        asciicast
            .stdout
            .into_iter()
            .map(|e| Ok(Event::output(e.time, e.data))),
    );

    Ok(Asciicast { header, events })
}
