use anyhow::Result;
use avt::Vt;
use tokio::time::{self, Duration, Instant};

use crate::asciicast::{self, Event, EventData};
use crate::config::Key;
use crate::tty::{DevTty, RawTty, TtySize};

const SEEK_SHORT: i64 = 5 * 1_000_000;
const SEEK_EXACT: i64 = 1_000_000;
const SEEK_MEDIUM: i64 = 60 * 1_000_000;
const SEEK_LONG: i64 = 10 * 60 * 1_000_000;
const ESCAPE_SEQUENCE_TIMEOUT: Duration = Duration::from_millis(10);
// Vt::dump() recreates state from a fresh terminal.
const REDRAW_PREFIX: &[u8] = b"\x1bc";

const LEFT_KEYS: &[&[u8]] = &[b"\x1b[D", b"\x1bOD"];
const RIGHT_KEYS: &[&[u8]] = &[b"\x1b[C", b"\x1bOC"];
const UP_KEYS: &[&[u8]] = &[b"\x1b[A", b"\x1bOA"];
const DOWN_KEYS: &[&[u8]] = &[b"\x1b[B", b"\x1bOB"];
const SHIFT_LEFT_KEYS: &[&[u8]] = &[b"\x1b[1;2D", b"\x1b[2D"];
const SHIFT_RIGHT_KEYS: &[&[u8]] = &[b"\x1b[1;2C", b"\x1b[2C"];
const SHIFT_UP_KEYS: &[&[u8]] = &[b"\x1b[1;2A", b"\x1b[2A"];
const SHIFT_DOWN_KEYS: &[&[u8]] = &[b"\x1b[1;2B", b"\x1b[2B"];
const HOME_KEYS: &[&[u8]] = &[b"\x1b[H", b"\x1bOH", b"\x1b[1~", b"\x1b[7~"];
const SHIFT_PAGE_UP_KEYS: &[&[u8]] = &[b"\x1b[5;2~", b"\x1b[5$"];
const SHIFT_PAGE_DOWN_KEYS: &[&[u8]] = &[b"\x1b[6;2~", b"\x1b[6$"];

pub struct KeyBindings {
    pub quit: Key,
    pub pause: Key,
    pub step: Key,
    pub step_back: Key,
    pub next_marker: Key,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            quit: Some(vec![0x03]),
            pause: Some(vec![b' ']),
            step: Some(vec![b'.']),
            step_back: Some(vec![b',']),
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
    let events = collect_session_events(recording, speed, idle_time_limit_override)?;
    let duration = events
        .last()
        .map(|event| event.time)
        .unwrap_or(Duration::ZERO);
    let mut epoch = Instant::now();
    let mut pause_elapsed_time: Option<Duration> = None;
    let mut next_event_idx = 0;
    let mut input = [0u8; 1024];
    let mut tty = DevTty::open().await?;
    let initial_size = if auto_resize {
        (initial_cols as usize, initial_rows as usize).into()
    } else {
        TtySize::from(tty.get_size())
    };
    let mut vt = build_vt(initial_size);

    if auto_resize {
        tty.resize(initial_size).await?;
    }

    while next_event_idx < events.len() {
        if let Some(pet) = pause_elapsed_time {
            let n = tty.read(&mut input).await?;
            let n = read_remaining_escape_sequence(&tty, &mut input, n).await?;
            let key = &input[..n];

            match key_action(key, keys) {
                Some(Action::Quit) => {
                    tty.write_all("\r\n".as_bytes()).await?;
                    return Ok(false);
                }

                Some(Action::TogglePause) => {
                    epoch = Instant::now() - pet;
                    pause_elapsed_time = None;
                }

                Some(Action::StepForward) => {
                    pause_elapsed_time = Some(
                        step_forward(&mut tty, &mut vt, &events, &mut next_event_idx, auto_resize)
                            .await?,
                    );
                }

                Some(Action::StepBackward) => {
                    pause_elapsed_time = Some(
                        step_backward(
                            &mut tty,
                            &mut vt,
                            &events,
                            &mut next_event_idx,
                            initial_size,
                            auto_resize,
                        )
                        .await?,
                    );
                }

                Some(Action::NextMarker) => {
                    pause_elapsed_time = Some(
                        step_to_next_marker(
                            &mut tty,
                            &mut vt,
                            &events,
                            &mut next_event_idx,
                            duration,
                            auto_resize,
                        )
                        .await?,
                    );
                }

                Some(Action::SeekStart) => {
                    seek_to_position(
                        &mut tty,
                        &mut vt,
                        &events,
                        &mut next_event_idx,
                        Duration::ZERO,
                        initial_size,
                        auto_resize,
                    )
                    .await?;
                    pause_elapsed_time = Some(Duration::ZERO);
                }

                Some(Action::SeekRelative(offset)) => {
                    let target = offset_position(pet, offset, duration);

                    seek_to_position(
                        &mut tty,
                        &mut vt,
                        &events,
                        &mut next_event_idx,
                        target,
                        initial_size,
                        auto_resize,
                    )
                    .await?;
                    pause_elapsed_time = Some(target);
                }

                None => {}
            }
        } else {
            while next_event_idx < events.len() {
                let event = &events[next_event_idx];
                let delay = event.time.as_micros() as i64 - epoch.elapsed().as_micros() as i64;

                if delay > 0 {
                    if let Ok(result) =
                        time::timeout(Duration::from_micros(delay as u64), tty.read(&mut input))
                            .await
                    {
                        let n = result?;
                        let n = read_remaining_escape_sequence(&tty, &mut input, n).await?;
                        let key = &input[..n];

                        match key_action(key, keys) {
                            Some(Action::Quit) => {
                                tty.write_all("\r\n".as_bytes()).await?;
                                return Ok(false);
                            }

                            Some(Action::TogglePause)
                            | Some(Action::StepForward)
                            | Some(Action::StepBackward) => {
                                pause_elapsed_time = Some(epoch.elapsed().min(duration));
                                break;
                            }

                            Some(Action::SeekStart) => {
                                seek_to_position(
                                    &mut tty,
                                    &mut vt,
                                    &events,
                                    &mut next_event_idx,
                                    Duration::ZERO,
                                    initial_size,
                                    auto_resize,
                                )
                                .await?;
                                epoch = Instant::now();
                                continue;
                            }

                            Some(Action::SeekRelative(offset)) => {
                                let target = offset_position(epoch.elapsed(), offset, duration);

                                seek_to_position(
                                    &mut tty,
                                    &mut vt,
                                    &events,
                                    &mut next_event_idx,
                                    target,
                                    initial_size,
                                    auto_resize,
                                )
                                .await?;
                                epoch = Instant::now() - target;
                                continue;
                            }

                            Some(Action::NextMarker) | None => {
                                continue;
                            }
                        }
                    }
                }

                apply_event(&mut tty, &mut vt, event, auto_resize).await?;
                next_event_idx += 1;

                if pause_on_markers && matches!(&event.data, EventData::Marker(_)) {
                    pause_elapsed_time = Some(event.time);
                    break;
                }
            }
        }
    }

    Ok(true)
}

fn collect_session_events(
    recording: asciicast::Asciicast<'static>,
    speed: f64,
    idle_time_limit_override: Option<f64>,
) -> Result<Vec<Event>> {
    let idle_time_limit = idle_time_limit_override
        .or(recording.header.idle_time_limit)
        .unwrap_or(f64::MAX);

    let events = asciicast::limit_idle_time(recording.events, idle_time_limit);
    let events = asciicast::accelerate(events, speed);

    events.collect()
}

async fn apply_event(
    tty: &mut DevTty,
    vt: &mut Vt,
    event: &Event,
    auto_resize: bool,
) -> Result<()> {
    match &event.data {
        EventData::Output(data) => {
            vt.feed_str(data);
            tty.write_all(data.as_bytes()).await?;
        }

        EventData::Resize(cols, rows) if auto_resize => {
            vt.resize(*cols as usize, *rows as usize);
            tty.resize((*cols as usize, *rows as usize).into()).await?;
        }

        _ => {}
    }

    Ok(())
}

fn apply_event_to_vt(vt: &mut Vt, event: &Event, auto_resize: bool) {
    match &event.data {
        EventData::Output(data) => {
            vt.feed_str(data);
        }

        EventData::Resize(cols, rows) if auto_resize => {
            vt.resize(*cols as usize, *rows as usize);
        }

        _ => {}
    }
}

async fn step_forward(
    tty: &mut DevTty,
    vt: &mut Vt,
    events: &[Event],
    next_event_idx: &mut usize,
    auto_resize: bool,
) -> Result<Duration> {
    if let Some(event) = events.get(*next_event_idx) {
        apply_event(tty, vt, event, auto_resize).await?;
        *next_event_idx += 1;

        Ok(event.time)
    } else {
        Ok(events
            .last()
            .map(|event| event.time)
            .unwrap_or(Duration::ZERO))
    }
}

async fn step_backward(
    tty: &mut DevTty,
    vt: &mut Vt,
    events: &[Event],
    next_event_idx: &mut usize,
    initial_size: TtySize,
    auto_resize: bool,
) -> Result<Duration> {
    let target_idx = next_event_idx.saturating_sub(1);
    seek_to_index(
        tty,
        vt,
        events,
        next_event_idx,
        target_idx,
        initial_size,
        auto_resize,
    )
    .await?;

    Ok(position_after_event(events, target_idx))
}

async fn step_to_next_marker(
    tty: &mut DevTty,
    vt: &mut Vt,
    events: &[Event],
    next_event_idx: &mut usize,
    duration: Duration,
    auto_resize: bool,
) -> Result<Duration> {
    while let Some(event) = events.get(*next_event_idx) {
        *next_event_idx += 1;

        if matches!(&event.data, EventData::Marker(_)) {
            return Ok(event.time);
        }

        apply_event(tty, vt, event, auto_resize).await?;
    }

    Ok(duration)
}

async fn seek_to_position(
    tty: &mut DevTty,
    vt: &mut Vt,
    events: &[Event],
    next_event_idx: &mut usize,
    position: Duration,
    initial_size: TtySize,
    auto_resize: bool,
) -> Result<()> {
    let target_idx = event_index_at(events, position);

    seek_to_index(
        tty,
        vt,
        events,
        next_event_idx,
        target_idx,
        initial_size,
        auto_resize,
    )
    .await
}

async fn seek_to_index(
    tty: &mut DevTty,
    vt: &mut Vt,
    events: &[Event],
    next_event_idx: &mut usize,
    target_idx: usize,
    initial_size: TtySize,
    auto_resize: bool,
) -> Result<()> {
    match target_idx.cmp(next_event_idx) {
        std::cmp::Ordering::Less => {
            *vt = build_vt(initial_size);

            for event in &events[..target_idx] {
                apply_event_to_vt(vt, event, auto_resize);
            }
        }

        std::cmp::Ordering::Equal => return Ok(()),

        std::cmp::Ordering::Greater => {
            for event in &events[*next_event_idx..target_idx] {
                apply_event_to_vt(vt, event, auto_resize);
            }
        }
    }

    if auto_resize {
        tty.resize(vt.size().into()).await?;
    }

    redraw(tty, vt).await?;
    *next_event_idx = target_idx;

    Ok(())
}

async fn redraw(tty: &mut DevTty, vt: &Vt) -> Result<()> {
    tty.write_all(REDRAW_PREFIX).await?;
    tty.write_all(vt.dump().as_bytes()).await?;

    Ok(())
}

async fn read_remaining_escape_sequence(
    tty: &DevTty,
    input: &mut [u8],
    mut len: usize,
) -> std::io::Result<usize> {
    if len > 0 && input[0] == b'\x1b' {
        while len < input.len()
            && !is_known_escape_key(&input[..len])
            && is_known_escape_key_prefix(&input[..len])
        {
            match time::timeout(ESCAPE_SEQUENCE_TIMEOUT, tty.read(&mut input[len..])).await {
                Ok(Ok(0)) | Err(_) => break,
                Ok(Ok(n)) => len += n,
                Ok(Err(e)) => return Err(e),
            }
        }
    }

    Ok(len)
}

fn build_vt(size: TtySize) -> Vt {
    Vt::builder()
        .size(size.0 as usize, size.1 as usize)
        .scrollback_limit(1000)
        .build()
}

fn event_index_at(events: &[Event], position: Duration) -> usize {
    events.partition_point(|event| event.time <= position)
}

fn position_after_event(events: &[Event], event_idx: usize) -> Duration {
    event_idx
        .checked_sub(1)
        .and_then(|idx| events.get(idx))
        .map(|event| event.time)
        .unwrap_or(Duration::ZERO)
}

fn offset_position(position: Duration, offset: i64, duration: Duration) -> Duration {
    if offset < 0 {
        position.saturating_sub(Duration::from_micros(offset.unsigned_abs()))
    } else {
        position
            .saturating_add(Duration::from_micros(offset as u64))
            .min(duration)
    }
}

#[derive(Debug, PartialEq)]
enum Action {
    Quit,
    TogglePause,
    StepForward,
    StepBackward,
    NextMarker,
    SeekStart,
    SeekRelative(i64),
}

fn key_action(key: &[u8], keys: &KeyBindings) -> Option<Action> {
    if keys.quit.as_ref().is_some_and(|k| k == key) {
        return Some(Action::Quit);
    }

    if keys.pause.as_ref().is_some_and(|k| k == key) {
        return Some(Action::TogglePause);
    }

    if keys.step.as_ref().is_some_and(|k| k == key) {
        return Some(Action::StepForward);
    }

    if keys.step_back.as_ref().is_some_and(|k| k == key) {
        return Some(Action::StepBackward);
    }

    if keys.next_marker.as_ref().is_some_and(|k| k == key) {
        return Some(Action::NextMarker);
    }

    if matches_any(key, HOME_KEYS) {
        Some(Action::SeekStart)
    } else if matches_any(key, LEFT_KEYS) {
        Some(Action::SeekRelative(-SEEK_SHORT))
    } else if matches_any(key, RIGHT_KEYS) {
        Some(Action::SeekRelative(SEEK_SHORT))
    } else if matches_any(key, UP_KEYS) {
        Some(Action::SeekRelative(SEEK_MEDIUM))
    } else if matches_any(key, DOWN_KEYS) {
        Some(Action::SeekRelative(-SEEK_MEDIUM))
    } else if matches_any(key, SHIFT_LEFT_KEYS) {
        Some(Action::SeekRelative(-SEEK_EXACT))
    } else if matches_any(key, SHIFT_RIGHT_KEYS) {
        Some(Action::SeekRelative(SEEK_EXACT))
    } else if matches_any(key, SHIFT_UP_KEYS) {
        Some(Action::SeekRelative(SEEK_SHORT))
    } else if matches_any(key, SHIFT_DOWN_KEYS) {
        Some(Action::SeekRelative(-SEEK_SHORT))
    } else if matches_any(key, SHIFT_PAGE_UP_KEYS) {
        Some(Action::SeekRelative(-SEEK_LONG))
    } else if matches_any(key, SHIFT_PAGE_DOWN_KEYS) {
        Some(Action::SeekRelative(SEEK_LONG))
    } else {
        None
    }
}

fn matches_any(key: &[u8], keys: &[&[u8]]) -> bool {
    keys.iter().any(|candidate| *candidate == key)
}

fn is_known_escape_key(key: &[u8]) -> bool {
    matches_any(key, LEFT_KEYS)
        || matches_any(key, RIGHT_KEYS)
        || matches_any(key, UP_KEYS)
        || matches_any(key, DOWN_KEYS)
        || matches_any(key, SHIFT_LEFT_KEYS)
        || matches_any(key, SHIFT_RIGHT_KEYS)
        || matches_any(key, SHIFT_UP_KEYS)
        || matches_any(key, SHIFT_DOWN_KEYS)
        || matches_any(key, HOME_KEYS)
        || matches_any(key, SHIFT_PAGE_UP_KEYS)
        || matches_any(key, SHIFT_PAGE_DOWN_KEYS)
}

fn is_known_escape_key_prefix(key: &[u8]) -> bool {
    has_prefix(key, LEFT_KEYS)
        || has_prefix(key, RIGHT_KEYS)
        || has_prefix(key, UP_KEYS)
        || has_prefix(key, DOWN_KEYS)
        || has_prefix(key, SHIFT_LEFT_KEYS)
        || has_prefix(key, SHIFT_RIGHT_KEYS)
        || has_prefix(key, SHIFT_UP_KEYS)
        || has_prefix(key, SHIFT_DOWN_KEYS)
        || has_prefix(key, HOME_KEYS)
        || has_prefix(key, SHIFT_PAGE_UP_KEYS)
        || has_prefix(key, SHIFT_PAGE_DOWN_KEYS)
}

fn has_prefix(key: &[u8], keys: &[&[u8]]) -> bool {
    keys.iter()
        .any(|candidate| candidate.len() > key.len() && candidate.starts_with(key))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{
        event_index_at, is_known_escape_key, is_known_escape_key_prefix, key_action,
        offset_position, position_after_event, Action, KeyBindings, SEEK_EXACT, SEEK_LONG,
        SEEK_MEDIUM, SEEK_SHORT,
    };
    use crate::asciicast::{Event, EventData};

    #[test]
    fn key_action_matches_mpv_seek_keys() {
        let keys = KeyBindings::default();

        assert_eq!(
            key_action(b"\x1b[D", &keys),
            Some(Action::SeekRelative(-SEEK_SHORT))
        );
        assert_eq!(
            key_action(b"\x1b[C", &keys),
            Some(Action::SeekRelative(SEEK_SHORT))
        );
        assert_eq!(
            key_action(b"\x1b[A", &keys),
            Some(Action::SeekRelative(SEEK_MEDIUM))
        );
        assert_eq!(
            key_action(b"\x1b[B", &keys),
            Some(Action::SeekRelative(-SEEK_MEDIUM))
        );
        assert_eq!(
            key_action(b"\x1b[1;2D", &keys),
            Some(Action::SeekRelative(-SEEK_EXACT))
        );
        assert_eq!(
            key_action(b"\x1b[1;2C", &keys),
            Some(Action::SeekRelative(SEEK_EXACT))
        );
        assert_eq!(
            key_action(b"\x1b[1;2A", &keys),
            Some(Action::SeekRelative(SEEK_SHORT))
        );
        assert_eq!(
            key_action(b"\x1b[1;2B", &keys),
            Some(Action::SeekRelative(-SEEK_SHORT))
        );
        assert_eq!(
            key_action(b"\x1b[5;2~", &keys),
            Some(Action::SeekRelative(-SEEK_LONG))
        );
        assert_eq!(
            key_action(b"\x1b[6;2~", &keys),
            Some(Action::SeekRelative(SEEK_LONG))
        );
        assert_eq!(key_action(b"\x1b[H", &keys), Some(Action::SeekStart));
    }

    #[test]
    fn key_action_matches_step_back_key() {
        let keys = KeyBindings::default();

        assert_eq!(key_action(b".", &keys), Some(Action::StepForward));
        assert_eq!(key_action(b",", &keys), Some(Action::StepBackward));
    }

    #[test]
    fn complete_known_escape_keys_do_not_need_more_input() {
        assert!(is_known_escape_key(b"\x1b[D"));
        assert!(is_known_escape_key(b"\x1b[1;2D"));
        assert!(is_known_escape_key_prefix(b"\x1b["));
        assert!(!is_known_escape_key_prefix(b"\x1b[D"));
        assert!(!is_known_escape_key_prefix(b"\x1b[999~"));
    }

    #[test]
    fn offset_position_saturates_to_recording_bounds() {
        let duration = Duration::from_secs(20);

        assert_eq!(
            offset_position(Duration::from_secs(10), 5_000_000, duration),
            Duration::from_secs(15)
        );
        assert_eq!(
            offset_position(Duration::from_secs(18), 5_000_000, duration),
            duration
        );
        assert_eq!(
            offset_position(Duration::from_secs(3), -5_000_000, duration),
            Duration::ZERO
        );
    }

    #[test]
    fn event_indexes_include_events_at_requested_position() {
        let events = events_at(&[1, 2, 2, 4]);

        assert_eq!(event_index_at(&events, Duration::from_secs(0)), 0);
        assert_eq!(event_index_at(&events, Duration::from_secs(1)), 1);
        assert_eq!(event_index_at(&events, Duration::from_secs(2)), 3);
        assert_eq!(event_index_at(&events, Duration::from_secs(3)), 3);
    }

    #[test]
    fn position_after_event_returns_last_rendered_event_time() {
        let events = events_at(&[1, 2, 4]);

        assert_eq!(position_after_event(&events, 0), Duration::ZERO);
        assert_eq!(position_after_event(&events, 1), Duration::from_secs(1));
        assert_eq!(position_after_event(&events, 3), Duration::from_secs(4));
    }

    fn events_at(times: &[u64]) -> Vec<Event> {
        times
            .iter()
            .map(|time| Event {
                time: Duration::from_secs(*time),
                data: EventData::Marker(String::new()),
            })
            .collect()
    }
}
