use std::collections::HashMap;
use std::time::Duration;

use anyhow::{bail, Result};
use serde::Deserialize;

use super::{Asciicast, Event, Header, Version};
use crate::asciicast::util::deserialize_time;

#[derive(Debug, Deserialize)]
struct V1 {
    version: u8,
    width: u16,
    height: u16,
    command: Option<String>,
    title: Option<String>,
    env: Option<HashMap<String, Option<String>>>,
    stdout: Vec<V1OutputEvent>,
}

#[derive(Debug, Deserialize)]
struct V1OutputEvent {
    #[serde(deserialize_with = "deserialize_time")]
    time: Duration,
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
        .map(|env| env.get("TERM"))
        .unwrap_or_default()
        .cloned()
        .unwrap_or_default();

    let env = asciicast.env.map(|env| {
        env.into_iter()
            .filter_map(|(k, v)| v.map(|v| (k, v)))
            .collect()
    });

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
        env,
    };

    let events = Box::new(asciicast.stdout.into_iter().scan(
        Duration::from_micros(0),
        |prev_time, event| {
            let time = *prev_time + event.time;
            *prev_time = time;

            Some(Ok(Event::output(time, event.data)))
        },
    ));

    Ok(Asciicast {
        version: Version::One,
        header,
        events,
    })
}
