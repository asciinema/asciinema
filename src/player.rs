use crate::config::Key;
use crate::format::asciicast::{self, Event, EventCode};
use crate::tty::Tty;
use anyhow::Result;
use nix::sys::select::{pselect, FdSet};
use nix::sys::time::{TimeSpec, TimeValLike};
use std::io::{self, Write};
use std::os::unix::io::AsRawFd;
use std::time::{Duration, Instant};

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
            pause: Some(vec![' ' as u8]),
            step: Some(vec!['.' as u8]),
            next_marker: Some(vec![']' as u8]),
        }
    }
}

pub fn play(
    recording: impl io::Read,
    mut tty: impl Tty,
    speed: f64,
    idle_time_limit: Option<f64>,
    pause_on_markers: bool,
    keys: &KeyBindings,
) -> Result<()> {
    let mut events = open_recording(recording, speed, idle_time_limit)?;
    let mut stdout = io::stdout();
    let mut epoch = Instant::now();
    let mut pause_elapsed_time: Option<u64> = None;
    let mut next_event = events.next().transpose()?;

    while let Some(Event { time, code, data }) = &next_event {
        if let Some(pet) = pause_elapsed_time {
            if let Some(input) = read_input(&mut tty, 1_000_000)? {
                if keys.quit.as_ref().is_some_and(|k| k == &input) {
                    stdout.write_all("\r\n".as_bytes())?;
                    return Ok(());
                }

                if keys.pause.as_ref().is_some_and(|k| k == &input) {
                    epoch = Instant::now() - Duration::from_micros(pet);
                    pause_elapsed_time = None;
                } else if keys.step.as_ref().is_some_and(|k| k == &input) {
                    pause_elapsed_time = Some(*time);

                    if code == &EventCode::Output {
                        stdout.write_all(data.as_bytes())?;
                        stdout.flush()?;
                    }

                    next_event = events.next().transpose()?;
                } else if keys.next_marker.as_ref().is_some_and(|k| k == &input) {
                    while let Some(Event { time, code, data }) = next_event {
                        next_event = events.next().transpose()?;

                        match code {
                            EventCode::Output => {
                                stdout.write_all(data.as_bytes())?;
                            }

                            EventCode::Marker => {
                                pause_elapsed_time = Some(time);
                                break;
                            }

                            _ => {}
                        }
                    }

                    stdout.flush()?;
                }
            }
        } else {
            while let Some(Event { time, code, data }) = &next_event {
                let delay = *time as i64 - epoch.elapsed().as_micros() as i64;

                if delay > 0 {
                    stdout.flush()?;

                    if let Some(key) = read_input(&mut tty, delay)? {
                        if keys.quit.as_ref().is_some_and(|k| k == &key) {
                            stdout.write_all("\r\n".as_bytes())?;
                            return Ok(());
                        }

                        if keys.pause.as_ref().is_some_and(|k| k == &key) {
                            pause_elapsed_time = Some(epoch.elapsed().as_micros() as u64);
                            break;
                        }

                        continue;
                    }
                }

                match code {
                    EventCode::Output => {
                        stdout.write_all(data.as_bytes())?;
                    }

                    EventCode::Marker => {
                        if pause_on_markers {
                            pause_elapsed_time = Some(*time);
                            next_event = events.next().transpose()?;
                            break;
                        }
                    }

                    _ => (),
                }

                next_event = events.next().transpose()?;
            }
        }
    }

    Ok(())
}

fn open_recording(
    recording: impl io::Read,
    speed: f64,
    idle_time_limit: Option<f64>,
) -> Result<impl Iterator<Item = Result<Event>>> {
    let reader = io::BufReader::new(recording);
    let (header, events) = asciicast::open(reader)?;

    let idle_time_limit = idle_time_limit
        .or(header.idle_time_limit)
        .unwrap_or(f64::MAX);

    let events = asciicast::limit_idle_time(events, idle_time_limit);
    let events = asciicast::accelerate(events, speed);

    Ok(events)
}

fn read_input<T: Tty>(tty: &mut T, timeout: i64) -> Result<Option<Vec<u8>>> {
    let nfds = Some(tty.as_fd().as_raw_fd() + 1);
    let mut rfds = FdSet::new();
    rfds.insert(tty);
    let timeout = TimeSpec::microseconds(timeout);
    let mut input: Vec<u8> = Vec::new();

    pselect(nfds, &mut rfds, None, None, &timeout, None)?;

    if rfds.contains(tty) {
        let mut buf = [0u8; 1024];

        while let Ok(n) = tty.read(&mut buf) {
            if n == 0 {
                break;
            }

            input.extend_from_slice(&buf[0..n]);
        }

        if input.len() > 0 {
            Ok(Some(input))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}
