use std::rc::Rc;
use std::time::Duration;

use anyhow::Result;
use avt::{Cell, Color, Pen, Vt};

use crate::asciicast::{Event, EventData, Header};

/// Lower and upper bounds applied to the terminal dimensions read from a
/// recording. avt panics on a zero dimension and allocates the whole grid
/// eagerly, so a crafted (or simply broken) cast must not be able to crash the
/// process or exhaust memory. No real terminal approaches the upper bound.
const MIN_DIMENSION: usize = 1;
const MAX_DIMENSION: usize = 2048;

pub struct Frame {
    pub number: usize,
    pub time: Duration,
    pub delta_time: Duration,
    pub changed_cells: usize,
    pub kind: FrameKind,
    pub screen: Rc<Screen>,
}

pub enum FrameKind {
    Output,
    Resize(u16, u16),
    Marker(String),
}

impl FrameKind {
    pub fn type_str(&self) -> &'static str {
        match self {
            FrameKind::Output => "output",
            FrameKind::Resize(..) => "resize",
            FrameKind::Marker(_) => "marker",
        }
    }
}

pub struct Screen {
    lines: Vec<Vec<Cell>>,
}

/// Whether an event produces a frame. Output and resize events change the
/// rendered screen; markers are listed for orientation. Input, exit, and other
/// events are not frames. This is the single definition of the frame set that
/// establishes the 1-based frame numbering used throughout.
fn is_frame_event(data: &EventData) -> bool {
    matches!(
        data,
        EventData::Output(_) | EventData::Resize(..) | EventData::Marker(_)
    )
}

fn clamp_size(cols: usize, rows: usize) -> (usize, usize) {
    (
        cols.clamp(MIN_DIMENSION, MAX_DIMENSION),
        rows.clamp(MIN_DIMENSION, MAX_DIMENSION),
    )
}

/// Replays a recording's events through a virtual terminal, yielding one
/// [`Frame`] per screen-affecting event. Events are consumed lazily from the
/// iterator, so the whole recording is never held in memory - only the virtual
/// terminal and the current/previous screen. Errors from the underlying event
/// stream are passed through; the caller should stop on the first one.
///
/// Event times are clamped to be non-decreasing. asciicast v3 already
/// guarantees monotonic timing; v1/v2 do not, and a player cannot travel
/// backwards, so a backward jump is treated as zero elapsed time. This keeps
/// time-based frame selection well-defined and streamable.
pub fn frames(
    header: &Header,
    events: impl Iterator<Item = Result<Event>>,
) -> impl Iterator<Item = Result<Frame>> {
    let (cols, rows) = clamp_size(header.term_cols as usize, header.term_rows as usize);

    let mut vt = Vt::builder().size(cols, rows).scrollback_limit(0).build();

    let mut prev_screen = Rc::new(Screen::from_vt(&vt));
    let mut prev_time = Duration::from_micros(0);
    let mut number = 0;

    events.filter_map(move |event| {
        let event = match event {
            Ok(event) if is_frame_event(&event.data) => event,
            Ok(_) => return None,
            Err(e) => return Some(Err(e)),
        };

        let time = event.time.max(prev_time);

        let kind = match &event.data {
            EventData::Output(text) => {
                vt.feed_str(text);

                FrameKind::Output
            }

            EventData::Resize(cols, rows) => {
                let (cols, rows) = clamp_size(*cols as usize, *rows as usize);
                vt.resize(cols, rows);

                FrameKind::Resize(cols as u16, rows as u16)
            }

            EventData::Marker(label) => FrameKind::Marker(label.clone()),

            _ => unreachable!("is_frame_event admitted a non-frame event"),
        };

        let screen = Rc::new(Screen::from_vt(&vt));
        let changed_cells = screen.changed_cells(&prev_screen);
        number += 1;

        let frame = Frame {
            number,
            time,
            delta_time: time - prev_time,
            changed_cells,
            kind,
            screen: Rc::clone(&screen),
        };

        prev_screen = screen;
        prev_time = time;

        Some(Ok(frame))
    })
}

/// Escapes control characters (including ESC, CR, and LF) in untrusted text so
/// it is safe to write to a terminal or a line-oriented format. Printable
/// characters, including non-ASCII, are left intact. Recording marker labels
/// are attacker-controlled and would otherwise inject escape sequences or split
/// output across lines.
pub fn escape_controls(s: &str) -> String {
    let mut out = String::with_capacity(s.len());

    for ch in s.chars() {
        if ch.is_control() {
            out.extend(ch.escape_default());
        } else {
            out.push(ch);
        }
    }

    out
}

impl Screen {
    fn from_vt(vt: &Vt) -> Self {
        Screen {
            lines: vt.view().map(|line| line.cells().to_vec()).collect(),
        }
    }

    pub fn cols(&self) -> usize {
        self.lines.first().map_or(0, |line| line.len())
    }

    pub fn rows(&self) -> usize {
        self.lines.len()
    }

    /// Returns the number of cells whose rendered contents (character or
    /// attributes) differ from the other screen. When the screens have
    /// different dimensions, cells missing on either side are treated as
    /// blank cells.
    pub fn changed_cells(&self, other: &Screen) -> usize {
        let empty: Vec<Cell> = Vec::new();
        let rows = self.lines.len().max(other.lines.len());
        let mut count = 0;

        for row in 0..rows {
            let a = self.lines.get(row).unwrap_or(&empty);
            let b = other.lines.get(row).unwrap_or(&empty);
            let cols = a.len().max(b.len());

            for col in 0..cols {
                let cell_a = a.get(col).copied().unwrap_or_default();
                let cell_b = b.get(col).copied().unwrap_or_default();

                if cell_a != cell_b {
                    count += 1;
                }
            }
        }

        count
    }

    /// Returns the screen lines as plain text, with trailing whitespace
    /// trimmed.
    pub fn text_lines(&self) -> Vec<String> {
        self.lines
            .iter()
            .map(|cells| {
                cells
                    .iter()
                    .filter(|cell| cell.width() > 0)
                    .map(|cell| cell.char())
                    .collect::<String>()
                    .trim_end()
                    .to_owned()
            })
            .collect()
    }

    /// Returns the screen lines with colors and text attributes reproduced
    /// with ANSI SGR sequences. Trailing blank cells are trimmed, and each
    /// line is self-contained - attributes are reset at the end of the line.
    pub fn seq_lines(&self) -> Vec<String> {
        self.lines.iter().map(|cells| seq_line(cells)).collect()
    }
}

fn seq_line(cells: &[Cell]) -> String {
    let trailers = cells.iter().rev().take_while(|c| c.is_default()).count();
    let mut line = String::new();

    // `None` means the default pen is in effect (the terminal's initial state),
    // so a leading run of default-pen cells emits no SGR at all - only a switch
    // to a non-default pen does.
    let mut pen: Option<&Pen> = None;

    for cell in cells[..cells.len() - trailers]
        .iter()
        .filter(|cell| cell.width() > 0)
    {
        let changed = match pen {
            Some(current) => current != cell.pen(),
            None => !cell.pen().is_default(),
        };

        if changed {
            line.push_str(&pen_seq(cell.pen()));
            pen = Some(cell.pen());
        }

        line.push(cell.char());
    }

    if pen.is_some_and(|pen| !pen.is_default()) {
        line.push_str("\x1b[0m");
    }

    line
}

fn pen_seq(pen: &Pen) -> String {
    let mut params = vec!["0".to_owned()];

    if pen.is_bold() {
        params.push("1".to_owned());
    }

    if pen.is_faint() {
        params.push("2".to_owned());
    }

    if pen.is_italic() {
        params.push("3".to_owned());
    }

    if pen.is_underline() {
        params.push("4".to_owned());
    }

    if pen.is_blink() {
        params.push("5".to_owned());
    }

    if pen.is_inverse() {
        params.push("7".to_owned());
    }

    if pen.is_strikethrough() {
        params.push("9".to_owned());
    }

    if let Some(color) = pen.foreground() {
        params.push(color_seq(color, 30));
    }

    if let Some(color) = pen.background() {
        params.push(color_seq(color, 40));
    }

    format!("\x1b[{}m", params.join(";"))
}

fn color_seq(color: Color, base: u8) -> String {
    match color {
        Color::Indexed(n) if n < 8 => (base + n).to_string(),
        Color::Indexed(n) if n < 16 => (base + 60 + n - 8).to_string(),
        Color::Indexed(n) => format!("{};5;{n}", base + 8),
        Color::RGB(c) => format!("{};2;{};{};{}", base + 8, c.r, c.g, c.b),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{escape_controls, Frame, FrameKind};
    use crate::asciicast::{Event, Header};

    fn header(cols: u16, rows: u16) -> Header {
        Header {
            term_cols: cols,
            term_rows: rows,
            ..Default::default()
        }
    }

    fn micros(time: u64) -> Duration {
        Duration::from_micros(time)
    }

    fn frames(header: &Header, events: Vec<Event>) -> Vec<Frame> {
        super::frames(header, events.into_iter().map(Ok))
            .collect::<anyhow::Result<Vec<_>>>()
            .unwrap()
    }

    #[test]
    fn output_frames() {
        let events = vec![
            Event::output(micros(500_000), "\x1b[31mred\x1b[0m".to_owned()),
            Event::output(micros(1_000_000), " text".to_owned()),
            Event::input(micros(1_200_000), "y".to_owned()),
            Event::output(micros(2_000_000), "\r\nbye".to_owned()),
        ];

        let frames = frames(&header(10, 4), events);

        assert_eq!(frames.len(), 3);

        assert_eq!(frames[0].number, 1);
        assert_eq!(frames[0].time, micros(500_000));
        assert_eq!(frames[0].delta_time, micros(500_000));
        assert_eq!(frames[0].changed_cells, 3);
        assert!(matches!(frames[0].kind, FrameKind::Output));

        // the space between "red" and "text" is written with a default pen
        // over a blank cell, so it doesn't count as changed
        assert_eq!(frames[1].number, 2);
        assert_eq!(frames[1].delta_time, micros(500_000));
        assert_eq!(frames[1].changed_cells, 4);

        // the input event is not a frame
        assert_eq!(frames[2].number, 3);
        assert_eq!(frames[2].time, micros(2_000_000));
        assert_eq!(frames[2].delta_time, micros(1_000_000));
        assert_eq!(frames[2].changed_cells, 3);

        assert_eq!(
            frames[2].screen.text_lines(),
            vec!["red text", "bye", "", ""]
        );
    }

    #[test]
    fn marker_frames() {
        let events = vec![
            Event::output(micros(1), "hello".to_owned()),
            Event::marker(micros(2), "step 1".to_owned()),
        ];

        let frames = frames(&header(10, 4), events);

        assert_eq!(frames.len(), 2);
        assert_eq!(frames[1].changed_cells, 0);
        assert!(matches!(frames[1].kind, FrameKind::Marker(ref l) if l == "step 1"));
    }

    #[test]
    fn resize_frames() {
        let events = vec![
            Event::output(micros(1), "red text".to_owned()),
            Event::resize(micros(2), (8, 3)),
            Event::resize(micros(3), (5, 3)),
        ];

        let frames = frames(&header(10, 4), events);

        assert_eq!(frames.len(), 3);

        // the content still fits after the resize - no cells changed
        assert_eq!(frames[1].changed_cells, 0);
        assert!(matches!(frames[1].kind, FrameKind::Resize(8, 3)));
        assert_eq!((frames[1].screen.cols(), frames[1].screen.rows()), (8, 3));

        // "red text" reflows into "red t" + "ext", and the first of these
        // lines scrolls out of the 3-row view - 7 cells of the first row
        // change ("red text" -> "ext")
        assert_eq!(frames[2].changed_cells, 7);
        assert_eq!(frames[2].screen.text_lines(), vec!["ext", "", ""]);
    }

    #[test]
    fn seq_lines() {
        let events = vec![Event::output(
            micros(1),
            "\x1b[1;31mred\x1b[0m plain".to_owned(),
        )];

        let frames = frames(&header(20, 2), events);
        let seq = frames[0].screen.seq_lines();

        assert_eq!(seq, vec!["\x1b[0;1;31mred\x1b[0m plain", ""]);
    }

    #[test]
    fn seq_lines_default_pen_has_no_leading_reset() {
        // a line that starts with default-pen cells must not be prefixed with a
        // redundant reset sequence
        let events = vec![Event::output(micros(1), "hello \x1b[31mred".to_owned())];

        let frames = frames(&header(20, 2), events);
        let seq = frames[0].screen.seq_lines();

        assert_eq!(seq, vec!["hello \x1b[0;31mred\x1b[0m", ""]);
    }

    #[test]
    fn tolerates_degenerate_dimensions() {
        // a recording declaring zero (or huge) terminal dimensions must not
        // panic or exhaust memory - the size is clamped to a sane range
        let events = vec![
            Event::output(micros(1), "hi".to_owned()),
            Event::resize(micros(2), (0, 0)),
            Event::output(micros(3), "there".to_owned()),
        ];

        let frames = frames(&header(0, 0), events);

        assert_eq!(frames.len(), 3);
        assert!(frames[1].screen.cols() >= 1 && frames[1].screen.rows() >= 1);
    }

    #[test]
    fn escapes_control_characters() {
        assert_eq!(escape_controls("plain żółć"), "plain żółć");
        assert_eq!(escape_controls("a\x1b]0;x\x07b"), "a\\u{1b}]0;x\\u{7}b");
        assert_eq!(escape_controls("line1\nline2"), "line1\\nline2");
    }
}
