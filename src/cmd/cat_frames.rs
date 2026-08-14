use std::io::{self, Write};
use std::path::Path;

use anyhow::Result;
use serde::Serialize;

use crate::asciicast::{self, Asciicast};
use crate::cli::{self, CatFramesFormat, FrameSpec, TimeSelector, TimeSpec};
use crate::frames::{self, Frame, FrameKind};
use crate::status;
use crate::util;

impl cli::CatFrames {
    pub fn run(self) -> Result<()> {
        if self.no_escapes && matches!(self.format, CatFramesFormat::Json) {
            status::warning!(
                "--no-escapes has no effect with --format json; both the text and escape renderings are always included"
            );
        }

        let input_path = self.get_input_path()?;
        let Asciicast { header, events, .. } =
            asciicast::open_from_path(input_path.as_ref().as_ref())?;

        // Stream frames through the selection instead of buffering the whole
        // recording; only the virtual terminal and a one-frame lookahead are
        // held at a time.
        let selection = Selection::new(&self.frame, &self.time);
        let frames = select(frames::frames(&header, events), selection);

        let mut writer = io::BufWriter::new(io::stdout().lock());

        let count = match self.format {
            CatFramesFormat::Terminal => write_terminal(frames, !self.no_escapes, &mut writer)?,
            CatFramesFormat::Json => write_json(frames, &mut writer)?,
        };

        writer.flush()?;

        if count == 0 {
            status::warning!("no frames matched the given --frame/--time selection");
        }

        Ok(())
    }

    fn get_input_path(&self) -> Result<Box<dyn AsRef<Path>>> {
        if self.file == "-" {
            Ok(Box::new(Path::new("/dev/stdin")))
        } else {
            util::get_local_path(&self.file)
        }
    }
}

enum TimeSel {
    Point(u128),
    Range(u128, u128),
}

/// A resolved set of frame selectors that can decide, for each frame in order,
/// whether it belongs to the selection - without materializing the frame list.
struct Selection {
    frame_ranges: Vec<(usize, usize)>,
    times: Vec<TimeSel>,
    max_frame: usize,
    max_time: Option<u128>,
}

impl Selection {
    fn new(frame_specs: &[FrameSpec], time_specs: &[TimeSpec]) -> Self {
        let frame_ranges: Vec<(usize, usize)> = frame_specs
            .iter()
            .flat_map(|spec| spec.0.iter().copied())
            .collect();

        let times: Vec<TimeSel> = time_specs
            .iter()
            .flat_map(|spec| spec.0.iter())
            .map(|selector| match selector {
                TimeSelector::Point(t) => TimeSel::Point(micros(*t)),
                TimeSelector::Range(a, b) => TimeSel::Range(micros(*a), micros(*b)),
            })
            .collect();

        let max_frame = frame_ranges.iter().map(|(_, end)| *end).max().unwrap_or(0);

        let max_time = times
            .iter()
            .map(|selector| match selector {
                TimeSel::Point(p) => *p,
                TimeSel::Range(_, b) => *b,
            })
            .max();

        Selection {
            frame_ranges,
            times,
            max_frame,
            max_time,
        }
    }

    /// Whether the frame at `number`/`time` is selected. `prev` and `next` are
    /// the times of the adjacent frames (`None` at the ends). Times are assumed
    /// non-decreasing, which `frames` guarantees.
    fn is_selected(
        &self,
        number: usize,
        time: u128,
        prev: Option<u128>,
        next: Option<u128>,
    ) -> bool {
        if self
            .frame_ranges
            .iter()
            .any(|(start, end)| number >= *start && number <= *end)
        {
            return true;
        }

        self.times.iter().any(|selector| match *selector {
            // The frame displayed at a point is the last one at or before it.
            TimeSel::Point(p) => time <= p && next.is_none_or(|n| n > p),

            TimeSel::Range(a, b) => {
                // last frame at or before the start, ...
                (time <= a && next.is_none_or(|n| n > a))
                    // ... every frame strictly inside, ...
                    || (time > a && time < b)
                    // ... and the first frame at or after the end.
                    || (time >= b && prev.is_none_or(|p| p < b))
            }
        })
    }

    /// Whether no later frame can match, so replay can stop after this one.
    fn done(&self, number: usize, time: u128) -> bool {
        number >= self.max_frame && self.max_time.is_none_or(|max| time >= max)
    }
}

/// Yields only the selected frames, in order, consuming the frame stream once
/// with a single-frame lookahead and stopping as soon as no later frame can
/// match.
fn select(
    frames: impl Iterator<Item = Result<Frame>>,
    selection: Selection,
) -> impl Iterator<Item = Result<Frame>> {
    let mut frames = frames.peekable();
    let mut prev_time: Option<u128> = None;
    let mut done = false;

    std::iter::from_fn(move || {
        if done {
            return None;
        }

        loop {
            let frame = match frames.next()? {
                Ok(frame) => frame,
                Err(e) => return Some(Err(e)),
            };

            let time = frame.time.as_micros();

            let next_time = match frames.peek() {
                Some(Ok(next)) => Some(next.time.as_micros()),
                _ => None,
            };

            let selected = selection.is_selected(frame.number, time, prev_time, next_time);
            done = selection.done(frame.number, time);
            prev_time = Some(time);

            if selected {
                return Some(Ok(frame));
            }

            if done {
                return None;
            }
        }
    })
}

fn micros(secs: f64) -> u128 {
    (secs * 1_000_000.0).round() as u128
}

fn write_terminal<W: Write>(
    frames: impl Iterator<Item = Result<Frame>>,
    escapes: bool,
    writer: &mut W,
) -> Result<usize> {
    let mut count = 0;

    for frame in frames {
        let frame = frame?;
        writeln!(writer, "{}", header_line(&frame))?;

        let mut lines = if escapes {
            frame.screen.seq_lines()
        } else {
            frame.screen.text_lines()
        };

        while lines.last().is_some_and(|line| line.is_empty()) {
            lines.pop();
        }

        for line in lines {
            writeln!(writer, "{line}")?;
        }

        count += 1;
    }

    Ok(count)
}

fn header_line(frame: &Frame) -> String {
    let annotation = match &frame.kind {
        FrameKind::Output => String::new(),
        FrameKind::Resize(cols, rows) => format!(" (resize: {cols}x{rows})"),
        FrameKind::Marker(label) => format!(" (marker: {})", frames::escape_controls(label)),
    };

    format!(
        "--- frame {} @ {:.6}{annotation} ---",
        frame.number,
        frame.time.as_secs_f64()
    )
}

#[derive(Serialize)]
struct FrameJson {
    frame: usize,
    time: f64,
    #[serde(rename = "type")]
    type_: &'static str,
    cols: usize,
    rows: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<String>,
    text: Vec<String>,
    seq: Vec<String>,
}

impl FrameJson {
    fn new(frame: &Frame) -> Self {
        FrameJson {
            frame: frame.number,
            time: frame.time.as_secs_f64(),
            type_: frame.kind.type_str(),
            cols: frame.screen.cols(),
            rows: frame.screen.rows(),
            label: match &frame.kind {
                FrameKind::Marker(label) => Some(label.clone()),
                _ => None,
            },
            text: frame.screen.text_lines(),
            seq: frame.screen.seq_lines(),
        }
    }
}

fn write_json<W: Write>(
    frames: impl Iterator<Item = Result<Frame>>,
    writer: &mut W,
) -> Result<usize> {
    let mut count = 0;

    for frame in frames {
        let frame = frame?;
        write!(writer, "{}", if count == 0 { "[\n" } else { ",\n" })?;
        write!(
            writer,
            "{}",
            serde_json::to_string(&FrameJson::new(&frame))?
        )?;
        count += 1;
    }

    if count == 0 {
        writeln!(writer, "[]")?;
    } else {
        writeln!(writer, "\n]")?;
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::cli::{FrameSpec, TimeSelector, TimeSpec};

    // frames-v3.cast has frame times 0.5, 1.0, 2.0, 3.0, 4.0 (an input event at
    // 2.5s is not a frame).
    fn all_frames() -> Vec<Frame> {
        let Asciicast { header, events, .. } =
            asciicast::open_from_path("tests/casts/frames-v3.cast").unwrap();

        frames::frames(&header, events)
            .map(|frame| frame.unwrap())
            .collect()
    }

    fn frame_spec(ranges: &[(usize, usize)]) -> Vec<FrameSpec> {
        vec![FrameSpec(ranges.to_vec())]
    }

    fn time_spec(selectors: &[TimeSelector]) -> Vec<TimeSpec> {
        vec![TimeSpec(selectors.to_vec())]
    }

    fn selected_numbers(frame_specs: Vec<FrameSpec>, time_specs: Vec<TimeSpec>) -> Vec<usize> {
        let selection = Selection::new(&frame_specs, &time_specs);

        select(all_frames().into_iter().map(Ok), selection)
            .map(|frame| frame.unwrap().number)
            .collect()
    }

    fn selected_frames(frame_specs: Vec<FrameSpec>, time_specs: Vec<TimeSpec>) -> Vec<Frame> {
        let selection = Selection::new(&frame_specs, &time_specs);

        select(all_frames().into_iter().map(Ok), selection)
            .map(|frame| frame.unwrap())
            .collect()
    }

    #[test]
    fn select_by_frame_number() {
        assert_eq!(
            selected_numbers(frame_spec(&[(1, 1), (3, 4)]), vec![]),
            vec![1, 3, 4]
        );

        // a range extending past the last frame simply yields what exists
        assert_eq!(
            selected_numbers(frame_spec(&[(4, 100)]), vec![]),
            vec![4, 5]
        );
    }

    #[test]
    fn select_by_time_point() {
        // the frame displayed at a time is the last one rendered at or before it
        assert_eq!(
            selected_numbers(vec![], time_spec(&[TimeSelector::Point(2.5)])),
            vec![3]
        );
        assert_eq!(
            selected_numbers(vec![], time_spec(&[TimeSelector::Point(2.0)])),
            vec![3]
        );

        // a time before the first frame selects nothing
        assert_eq!(
            selected_numbers(vec![], time_spec(&[TimeSelector::Point(0.4)])),
            Vec::<usize>::new()
        );
    }

    #[test]
    fn select_by_time_range() {
        // frames within the range, plus the frame displayed at its start and the
        // first frame rendered at or after its end
        assert_eq!(
            selected_numbers(vec![], time_spec(&[TimeSelector::Range(1.5, 2.5)])),
            vec![2, 3, 4]
        );

        // exact boundary frames are included without extending further
        assert_eq!(
            selected_numbers(vec![], time_spec(&[TimeSelector::Range(1.0, 2.0)])),
            vec![2, 3]
        );

        // a range past the last frame has no frame at or after its end
        assert_eq!(
            selected_numbers(vec![], time_spec(&[TimeSelector::Range(3.5, 100.0)])),
            vec![4, 5]
        );
    }

    #[test]
    fn select_unions_frame_and_time() {
        // combined selectors are the union, printed once in frame order
        assert_eq!(
            selected_numbers(
                frame_spec(&[(1, 1)]),
                time_spec(&[TimeSelector::Point(2.5)])
            ),
            vec![1, 3]
        );
    }

    #[test]
    fn last_frame_is_selectable_by_time() {
        // the final frame has no successor; a point at or after it still selects it
        assert_eq!(
            selected_numbers(vec![], time_spec(&[TimeSelector::Point(9.0)])),
            vec![5]
        );
    }

    #[test]
    fn terminal_output() {
        let mut output = Vec::new();
        let frames = selected_frames(frame_spec(&[(1, 1), (3, 3)]), vec![]);

        write_terminal(frames.into_iter().map(Ok), false, &mut output).unwrap();

        let output = String::from_utf8(output).unwrap();

        assert_eq!(
            output,
            "--- frame 1 @ 0.500000 ---\n\
             red\n\
             --- frame 3 @ 2.000000 (marker: step 1) ---\n\
             red text\n"
        );
    }

    #[test]
    fn terminal_output_with_escapes() {
        let mut output = Vec::new();
        let frames = selected_frames(frame_spec(&[(1, 1)]), vec![]);

        write_terminal(frames.into_iter().map(Ok), true, &mut output).unwrap();

        let output = String::from_utf8(output).unwrap();

        assert_eq!(output, "--- frame 1 @ 0.500000 ---\n\x1b[0;31mred\x1b[0m\n");
    }

    #[test]
    fn header_line_escapes_marker_labels() {
        let frame = Frame {
            number: 1,
            time: Duration::from_secs(1),
            delta_time: Duration::from_secs(1),
            changed_cells: 0,
            kind: FrameKind::Marker("a\x1b]0;x\x07b".to_owned()),
            screen: all_frames().pop().unwrap().screen,
        };

        let header = header_line(&frame);

        assert!(!header.contains('\x1b'), "escape byte leaked: {header:?}");
        assert!(header.contains("\\u{1b}]0;x\\u{7}b"));
    }

    #[test]
    fn json_output() {
        let mut output = Vec::new();
        let frames = selected_frames(frame_spec(&[(1, 1), (4, 4)]), vec![]);

        write_json(frames.into_iter().map(Ok), &mut output).unwrap();

        let frames: serde_json::Value = serde_json::from_slice(&output).unwrap();

        assert_eq!(frames[0]["frame"], 1);
        assert_eq!(frames[0]["time"], 0.5);
        assert_eq!(frames[0]["type"], "output");
        assert_eq!(frames[0]["cols"], 10);
        assert_eq!(frames[0]["rows"], 4);
        assert_eq!(frames[0]["text"][0], "red");
        assert_eq!(frames[0]["seq"][0], "\u{1b}[0;31mred\u{1b}[0m");

        assert_eq!(frames[1]["frame"], 4);
        assert_eq!(frames[1]["type"], "resize");
        assert_eq!(frames[1]["cols"], 8);
        assert_eq!(frames[1]["rows"], 3);

        assert_eq!(frames.as_array().unwrap().len(), 2);
    }

    #[test]
    fn json_output_empty() {
        let mut output = Vec::new();

        write_json(std::iter::empty(), &mut output).unwrap();

        assert_eq!(output, b"[]\n");
    }
}
