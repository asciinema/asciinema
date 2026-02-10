use anyhow::Result;
use tokio::sync::mpsc;
use tokio::time::{self, Duration, Instant};

use crate::asciicast::{self, Event, EventData};
use crate::config::Key;
use crate::tty::{DevTty, RawTty};

pub struct KeyBindings {
    pub quit: Key,
    pub pause: Key,
    pub step: Key,
    pub next_marker: Key,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            quit: Some(vec![0x03]),
            pause: Some(vec![b' ']),
            step: Some(vec![b'.']),
            next_marker: Some(vec![b']']),
        }
    }
}

pub async fn play(
    recording: asciicast::Asciicast<'static>,
    speed: f64,
    idle_time_limit_override: Option<f64>,
    pause_on_markers: bool,
    keys: &KeyBindings,
    auto_resize: bool,
) -> Result<bool> {
    let initial_cols = recording.header.term_cols;
    let initial_rows = recording.header.term_rows;
    let mut events = emit_session_events(recording, speed, idle_time_limit_override)?;
    let mut epoch = Instant::now();
    let mut pause_elapsed_time: Option<u64> = None;
    let mut next_event = events.recv().await.transpose()?;
    let mut input = [0u8; 1024];
    let mut tty = DevTty::open().await?;

    if auto_resize {
        tty.resize((initial_cols as usize, initial_rows as usize).into())
            .await?;
    }

    while let Some(Event { time, data }) = &next_event {
        if let Some(pet) = pause_elapsed_time {
            let n = tty.read(&mut input).await?;
            let key = &input[..n];

            if keys.quit.as_ref().is_some_and(|k| k == key) {
                tty.write_all("\r\n".as_bytes()).await?;
                return Ok(false);
            }

            if keys.pause.as_ref().is_some_and(|k| k == key) {
                epoch = Instant::now() - Duration::from_micros(pet);
                pause_elapsed_time = None;
            } else if keys.step.as_ref().is_some_and(|k| k == key) {
                pause_elapsed_time = Some(time.as_micros() as u64);

                match data {
                    EventData::Output(data) => {
                        tty.write_all(data.as_bytes()).await?;
                    }

                    EventData::Resize(cols, rows) if auto_resize => {
                        tty.resize((*cols as usize, *rows as usize).into()).await?;
                    }

                    _ => {}
                }

                next_event = events.recv().await.transpose()?;
            } else if keys.next_marker.as_ref().is_some_and(|k| k == key) {
                while let Some(Event { time, data }) = next_event {
                    next_event = events.recv().await.transpose()?;

                    match data {
                        EventData::Output(data) => {
                            tty.write_all(data.as_bytes()).await?;
                        }

                        EventData::Marker(_) => {
                            pause_elapsed_time = Some(time.as_micros() as u64);
                            break;
                        }

                        EventData::Resize(cols, rows) if auto_resize => {
                            tty.resize((cols as usize, rows as usize).into()).await?;
                        }

                        _ => {}
                    }
                }
            }
        } else {
            while let Some(Event { time, data }) = &next_event {
                let delay = time.as_micros() as i64 - epoch.elapsed().as_micros() as i64;

                if delay > 0 {
                    if let Ok(result) =
                        time::timeout(Duration::from_micros(delay as u64), tty.read(&mut input))
                            .await
                    {
                        let n = result?;
                        let key = &input[..n];

                        if keys.quit.as_ref().is_some_and(|k| k == key) {
                            tty.write_all("\r\n".as_bytes()).await?;
                            return Ok(false);
                        }

                        if keys.pause.as_ref().is_some_and(|k| k == key) {
                            pause_elapsed_time = Some(epoch.elapsed().as_micros() as u64);
                            break;
                        }

                        continue;
                    }
                }

                match data {
                    EventData::Output(data) => {
                        tty.write_all(data.as_bytes()).await?;
                    }

                    EventData::Resize(cols, rows) if auto_resize => {
                        tty.resize((*cols as usize, *rows as usize).into()).await?;
                    }

                    EventData::Marker(_) => {
                        if pause_on_markers {
                            pause_elapsed_time = Some(time.as_micros() as u64);
                            next_event = events.recv().await.transpose()?;
                            break;
                        }
                    }

                    _ => (),
                }

                next_event = events.recv().await.transpose()?;
            }
        }
    }

    Ok(true)
}

fn emit_session_events(
    recording: asciicast::Asciicast<'static>,
    speed: f64,
    idle_time_limit_override: Option<f64>,
) -> Result<mpsc::Receiver<Result<Event>>> {
    let idle_time_limit = idle_time_limit_override
        .or(recording.header.idle_time_limit)
        .unwrap_or(f64::MAX);

    let events = asciicast::limit_idle_time(recording.events, idle_time_limit);
    let events = asciicast::accelerate(events, speed);
    let (tx, rx) = mpsc::channel::<Result<Event>>(1024);

    tokio::task::spawn_blocking(move || {
        for event in events {
            if tx.blocking_send(event).is_err() {
                break;
            }
        }
    });

    Ok(rx)
}
