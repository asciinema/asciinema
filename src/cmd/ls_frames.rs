use std::io::{self, Write};
use std::path::Path;

use anyhow::Result;
use serde::Serialize;

use crate::asciicast::{self, Asciicast};
use crate::cli::{self, ListFormat};
use crate::frames::{self, Frame, FrameKind};
use crate::util;

impl cli::LsFrames {
    pub fn run(self) -> Result<()> {
        let input_path = self.get_input_path()?;
        let Asciicast { header, events, .. } =
            asciicast::open_from_path(input_path.as_ref().as_ref())?;
        let rows = frames::frames(&header, events)
            .map(|frame| frame.map(Row::from))
            .collect::<Result<Vec<Row>>>()?;
        let mut writer = io::BufWriter::new(io::stdout().lock());

        match self.format {
            ListFormat::Table => write_table(&rows, &mut writer)?,
            ListFormat::Csv => write_csv(&rows, &mut writer)?,
            ListFormat::Json => write_json(&rows, &mut writer)?,
        }

        writer.flush()?;

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

#[derive(Serialize)]
struct Row {
    frame: usize,
    time: f64,
    delta_time: f64,
    delta_cells: usize,
    #[serde(rename = "type")]
    type_: &'static str,
    // The terminal dimensions in effect at this frame. Reported for every frame
    // (in both this command and cat-frames) so the key has one meaning.
    cols: usize,
    rows: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<String>,
    // Human-readable summary for the table/CSV DETAIL column. Escaped for safe
    // display and excluded from JSON, which carries the structured fields.
    #[serde(skip)]
    detail: String,
}

impl From<Frame> for Row {
    fn from(frame: Frame) -> Self {
        let type_ = frame.kind.type_str();
        let (cols, rows) = (frame.screen.cols(), frame.screen.rows());

        let label = match &frame.kind {
            FrameKind::Marker(label) => Some(label.clone()),
            _ => None,
        };

        let detail = match &frame.kind {
            FrameKind::Output => String::new(),
            FrameKind::Resize(cols, rows) => format!("{cols}x{rows}"),
            FrameKind::Marker(label) => frames::escape_controls(label),
        };

        Row {
            frame: frame.number,
            time: frame.time.as_secs_f64(),
            delta_time: frame.delta_time.as_secs_f64(),
            delta_cells: frame.changed_cells,
            type_,
            cols,
            rows,
            label,
            detail,
        }
    }
}

fn write_table<W: Write>(rows: &[Row], writer: &mut W) -> Result<()> {
    let cells: Vec<[String; 6]> = rows
        .iter()
        .map(|row| {
            [
                row.frame.to_string(),
                format!("{:.6}", row.time),
                format!("{:.6}", row.delta_time),
                row.delta_cells.to_string(),
                row.type_.to_owned(),
                row.detail.clone(),
            ]
        })
        .collect();

    let headers = ["FRAME", "TIME", "DELTA-T", "DELTA-CELLS", "TYPE", "DETAIL"];

    let widths: Vec<usize> = headers
        .iter()
        .enumerate()
        .map(|(i, header)| {
            cells
                .iter()
                .map(|row| row[i].len())
                .max()
                .unwrap_or(0)
                .max(header.len())
        })
        .collect();

    let format_line = |cols: [&str; 6]| {
        let line = format!(
            "{:>fw$}  {:>tw$}  {:>dw$}  {:>cw$}  {:<yw$}  {}",
            cols[0],
            cols[1],
            cols[2],
            cols[3],
            cols[4],
            cols[5],
            fw = widths[0],
            tw = widths[1],
            dw = widths[2],
            cw = widths[3],
            yw = widths[4],
        );

        line.trim_end().to_owned()
    };

    writeln!(writer, "{}", format_line(headers))?;

    for row in &cells {
        let cols: [&str; 6] = std::array::from_fn(|i| row[i].as_str());
        writeln!(writer, "{}", format_line(cols))?;
    }

    Ok(())
}

fn write_csv<W: Write>(rows: &[Row], writer: &mut W) -> Result<()> {
    writeln!(writer, "frame,time,delta_time,delta_cells,type,detail")?;

    for row in rows {
        writeln!(
            writer,
            "{},{:.6},{:.6},{},{},{}",
            row.frame,
            row.time,
            row.delta_time,
            row.delta_cells,
            row.type_,
            csv_escape(&row.detail)
        )?;
    }

    Ok(())
}

fn csv_escape(value: &str) -> String {
    // Prefix a single quote to neutralize spreadsheet formula injection
    // (CWE-1236) for a value a spreadsheet would evaluate as a formula. The
    // detail column can contain an attacker-controlled marker label.
    let value = if value.starts_with(['=', '+', '-', '@']) {
        format!("'{value}")
    } else {
        value.to_owned()
    };

    if value.contains(['"', ',', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value
    }
}

fn write_json<W: Write>(rows: &[Row], writer: &mut W) -> Result<()> {
    if rows.is_empty() {
        writeln!(writer, "[]")?;

        return Ok(());
    }

    writeln!(writer, "[")?;

    for (i, row) in rows.iter().enumerate() {
        let separator = if i + 1 < rows.len() { "," } else { "" };
        writeln!(writer, "{}{separator}", serde_json::to_string(row)?)?;
    }

    writeln!(writer, "]")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_rows() -> Vec<Row> {
        let Asciicast { header, events, .. } =
            asciicast::open_from_path("tests/casts/frames-v3.cast").unwrap();

        frames::frames(&header, events)
            .map(|frame| Row::from(frame.unwrap()))
            .collect()
    }

    #[test]
    fn table() {
        let mut output = Vec::new();

        write_table(&fixture_rows(), &mut output).unwrap();

        let output = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output.lines().collect();

        assert_eq!(
            lines[0],
            "FRAME      TIME   DELTA-T  DELTA-CELLS  TYPE    DETAIL"
        );

        assert_eq!(lines[1], "    1  0.500000  0.500000            3  output");
        assert_eq!(
            lines[3],
            "    3  2.000000  1.000000            0  marker  step 1"
        );
        assert_eq!(
            lines[4],
            "    4  3.000000  1.000000            0  resize  8x3"
        );
        assert_eq!(lines.len(), 6);
    }

    #[test]
    fn csv() {
        let mut output = Vec::new();

        write_csv(&fixture_rows(), &mut output).unwrap();

        let output = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = output.lines().collect();

        assert_eq!(lines[0], "frame,time,delta_time,delta_cells,type,detail");
        assert_eq!(lines[1], "1,0.500000,0.500000,3,output,");
        assert_eq!(lines[3], "3,2.000000,1.000000,0,marker,step 1");
        assert_eq!(lines[4], "4,3.000000,1.000000,0,resize,8x3");
    }

    #[test]
    fn csv_escaping() {
        assert_eq!(csv_escape("plain"), "plain");
        assert_eq!(csv_escape("a,b"), "\"a,b\"");
        assert_eq!(csv_escape("say \"hi\""), "\"say \"\"hi\"\"\"");
        // spreadsheet formula injection is neutralized with a leading quote
        assert_eq!(csv_escape("=cmd|calc"), "'=cmd|calc");
        assert_eq!(csv_escape("+1"), "'+1");
        assert_eq!(csv_escape("@SUM(A1)"), "'@SUM(A1)");
    }

    #[test]
    fn json() {
        let mut output = Vec::new();

        write_json(&fixture_rows(), &mut output).unwrap();

        let rows: serde_json::Value = serde_json::from_slice(&output).unwrap();

        assert_eq!(rows[0]["frame"], 1);
        assert_eq!(rows[0]["time"], 0.5);
        assert_eq!(rows[0]["delta_cells"], 3);
        assert_eq!(rows[0]["type"], "output");
        assert!(rows[0].get("label").is_none());
        // cols/rows report the current screen size for every frame
        assert_eq!(rows[0]["cols"], 10);
        assert_eq!(rows[0]["rows"], 4);
        // detail is a display-only column, not part of the JSON
        assert!(rows[0].get("detail").is_none());

        assert_eq!(rows[2]["type"], "marker");
        assert_eq!(rows[2]["label"], "step 1");

        assert_eq!(rows[3]["type"], "resize");
        assert_eq!(rows[3]["cols"], 8);
        assert_eq!(rows[3]["rows"], 3);

        assert_eq!(rows.as_array().unwrap().len(), 5);
    }
}
