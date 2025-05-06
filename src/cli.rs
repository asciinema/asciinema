use std::net::SocketAddr;
use std::num::ParseIntError;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

pub const DEFAULT_LISTEN_ADDR: &str = "127.0.0.1:8080";

#[derive(Debug, Parser)]
#[clap(author, version, about)]
#[command(name = "asciinema")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// asciinema server URL
    #[arg(long, global = true, display_order = 100, value_name = "URL")]
    pub server_url: Option<String>,

    /// Quiet mode, i.e. suppress diagnostic messages
    #[clap(short, long, global = true, display_order = 101)]
    pub quiet: bool,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Record a terminal session
    Rec(Record),

    /// Replay a terminal session
    Play(Play),

    /// Stream a terminal session
    Stream(Stream),

    /// Record and/or stream a terminal session
    Session(Session),

    /// Concatenate multiple recordings
    Cat(Cat),

    /// Convert a recording into another format
    Convert(Convert),

    /// Upload a recording to an asciinema server
    Upload(Upload),

    /// Authenticate this CLI with an asciinema server account
    Auth(Auth),
}

#[derive(Debug, Args)]
pub struct Record {
    /// Output file path
    pub output_path: String,

    /// Output file format [default: asciicast-v3]
    #[arg(short = 'f', long, value_enum, value_name = "FORMAT")]
    pub output_format: Option<Format>,

    /// Command to start in the session [default: $SHELL]
    #[arg(short, long)]
    pub command: Option<String>,

    /// Enable input (keys) recording
    #[arg(long, short = 'I', alias = "stdin")]
    pub rec_input: bool,

    /// Comma-separated list of env vars to capture [default: SHELL]
    #[arg(long, value_name = "VARS")]
    pub rec_env: Option<String>,

    /// Append to an existing recording file
    #[arg(short, long)]
    pub append: bool,

    /// Overwrite output file if it already exists
    #[arg(long, conflicts_with = "append")]
    pub overwrite: bool,

    /// Title of the recording
    #[arg(short, long)]
    pub title: Option<String>,

    /// Limit idle time to a given number of seconds
    #[arg(short, long, value_name = "SECS")]
    pub idle_time_limit: Option<f64>,

    /// Headless mode, i.e. don't use TTY for input/output
    #[arg(long)]
    pub headless: bool,

    /// Override terminal size for the session
    #[arg(long, value_name = "COLSxROWS", value_parser = parse_tty_size)]
    pub tty_size: Option<(Option<u16>, Option<u16>)>,

    #[arg(long, hide = true)]
    pub cols: Option<u16>,

    #[arg(long, hide = true)]
    pub rows: Option<u16>,

    #[arg(long, hide = true)]
    pub raw: bool,
}

#[derive(Debug, Args)]
pub struct Play {
    #[arg(value_name = "FILENAME_OR_URL")]
    pub filename: String,

    /// Limit idle time to a given number of seconds
    #[arg(short, long, value_name = "SECS")]
    pub idle_time_limit: Option<f64>,

    /// Set playback speed
    #[arg(short, long)]
    pub speed: Option<f64>,

    /// Loop loop loop loop
    #[arg(short, long, name = "loop")]
    pub loop_: bool,

    /// Automatically pause on markers
    #[arg(short = 'm', long)]
    pub pause_on_markers: bool,
}

#[derive(Debug, Args)]
pub struct Stream {
    /// Stream the session via a local HTTP server
    #[arg(short, long, value_name = "IP:PORT", default_missing_value = DEFAULT_LISTEN_ADDR, num_args = 0..=1)]
    pub local: Option<SocketAddr>,

    /// Stream the session via a remote asciinema server
    #[arg(short, long, value_name = "STREAM-ID|WS-URL", default_missing_value = "", num_args = 0..=1, value_parser = validate_forward_target)]
    pub remote: Option<RelayTarget>,

    /// Command to start in the session [default: $SHELL]
    #[arg(short, long)]
    pub command: Option<String>,

    /// Enable input (keys) recording
    #[arg(long, short = 'I')]
    pub rec_input: bool,

    /// Comma-separated list of env vars to capture [default: SHELL]
    #[arg(long, value_name = "VARS")]
    pub rec_env: Option<String>,

    /// Headless mode, i.e. don't use TTY for input/output
    #[arg(long)]
    pub headless: bool,

    /// Override terminal size for the session
    #[arg(long, value_name = "COLSxROWS", value_parser = parse_tty_size)]
    pub tty_size: Option<(Option<u16>, Option<u16>)>,

    /// Log file path
    #[arg(long, value_name = "PATH")]
    pub log_file: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct Session {
    /// Save the session in a file
    #[arg(short, long, value_name = "PATH")]
    pub output_file: Option<String>,

    /// Output file format [default: asciicast-v3]
    #[arg(short = 'f', long, value_enum, value_name = "FORMAT")]
    pub output_format: Option<Format>,

    /// Stream the session via a local HTTP server
    #[arg(short = 'l', long, value_name = "IP:PORT", default_missing_value = DEFAULT_LISTEN_ADDR, num_args = 0..=1)]
    pub stream_local: Option<SocketAddr>,

    /// Stream the session via a remote asciinema server
    #[arg(short = 'r', long, value_name = "STREAM-ID|WS-URL", default_missing_value = "", num_args = 0..=1, value_parser = validate_forward_target)]
    pub stream_remote: Option<RelayTarget>,

    /// Command to start in the session [default: $SHELL]
    #[arg(short, long)]
    pub command: Option<String>,

    /// Enable input (keys) recording
    #[arg(long, short = 'I')]
    pub rec_input: bool,

    /// Comma-separated list of env vars to capture [default: SHELL]
    #[arg(long, value_name = "VARS")]
    pub rec_env: Option<String>,

    /// Append to an existing recording file
    #[arg(short, long)]
    pub append: bool,

    /// Overwrite output file if it already exists
    #[arg(long, conflicts_with = "append")]
    pub overwrite: bool,

    /// Title of the recording
    #[arg(short, long)]
    pub title: Option<String>,

    /// Limit idle time to a given number of seconds
    #[arg(short, long, value_name = "SECS")]
    pub idle_time_limit: Option<f64>,

    /// Headless mode, i.e. don't use TTY for input/output
    #[arg(long)]
    pub headless: bool,

    /// Override terminal size for the session
    #[arg(long, value_name = "COLSxROWS", value_parser = parse_tty_size)]
    pub tty_size: Option<(Option<u16>, Option<u16>)>,

    /// Log file path
    #[arg(long, value_name = "PATH")]
    pub log_file: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct Cat {
    #[arg(required = true)]
    pub filename: Vec<String>,
}

#[derive(Debug, Args)]
pub struct Convert {
    /// File to convert from, in asciicast format (use - for stdin)
    #[arg(value_name = "INPUT_FILENAME_OR_URL")]
    pub input_filename: String,

    /// File to convert to (use - for stdout)
    pub output_filename: String,

    /// Output file format [default: asciicast-v3]
    #[arg(short = 'f', long, value_enum, value_name = "FORMAT")]
    pub output_format: Option<Format>,

    /// Overwrite target file if it already exists
    #[arg(long)]
    pub overwrite: bool,
}

#[derive(Debug, Args)]
pub struct Upload {
    /// Filename/path of asciicast to upload
    pub filename: String,
}

#[derive(Debug, Args)]
pub struct Auth {}

#[derive(Clone, Copy, Debug, PartialEq, ValueEnum)]
pub enum Format {
    AsciicastV3,
    AsciicastV2,
    Raw,
    Txt,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum RelayTarget {
    StreamId(String),
    WsProducerUrl(url::Url),
}

fn parse_tty_size(s: &str) -> Result<(Option<u16>, Option<u16>), String> {
    match s.split_once('x') {
        Some((cols, "")) => {
            let cols: u16 = cols.parse().map_err(|e: ParseIntError| e.to_string())?;

            Ok((Some(cols), None))
        }

        Some(("", rows)) => {
            let rows: u16 = rows.parse().map_err(|e: ParseIntError| e.to_string())?;

            Ok((None, Some(rows)))
        }

        Some((cols, rows)) => {
            let cols: u16 = cols.parse().map_err(|e: ParseIntError| e.to_string())?;
            let rows: u16 = rows.parse().map_err(|e: ParseIntError| e.to_string())?;

            Ok((Some(cols), Some(rows)))
        }

        None => Err(s.to_owned()),
    }
}

fn validate_forward_target(s: &str) -> Result<RelayTarget, String> {
    let s = s.trim();

    match url::Url::parse(s) {
        Ok(url) => {
            let scheme = url.scheme();

            if scheme == "ws" || scheme == "wss" {
                Ok(RelayTarget::WsProducerUrl(url))
            } else {
                Err("must be a WebSocket URL (ws:// or wss://)".to_owned())
            }
        }

        Err(url::ParseError::RelativeUrlWithoutBase) => Ok(RelayTarget::StreamId(s.to_owned())),
        Err(e) => Err(e.to_string()),
    }
}
