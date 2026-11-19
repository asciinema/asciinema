use std::net::SocketAddr;
use std::num::ParseIntError;
use std::path::PathBuf;

use clap::{ArgGroup, Args, Parser, Subcommand, ValueEnum};

pub const DEFAULT_LISTEN_ADDR: &str = "127.0.0.1:0";

#[derive(Debug, Parser)]
#[clap(author, version, about)]
#[command(name = "asciinema", max_term_width = 100, infer_subcommands = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Suppress diagnostic messages and progress indicators. Only error messages will be displayed.
    #[clap(
        short,
        long,
        global = true,
        display_order = 101,
        help = "Quiet mode - suppress diagnostic messages",
        long_help
    )]
    pub quiet: bool,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Record a terminal session to a file.
    ///
    /// Captures all terminal output and optionally keyboard input, saving it for later playback. Supports various output formats, idle time limiting, and session customization options.
    ///
    /// Press <ctrl+d> or type 'exit' to end the recording session.
    /// Press <ctrl+\> to pause/resume capture of the session.
    ///
    /// During the session, the ASCIINEMA_SESSION environment variable is set to a unique session ID.
    #[clap(
        visible_alias = "rec",
        about = "Record a terminal session",
        long_about,
        after_help = "\x1b[1;4mExamples\x1b[0m:

  asciinema rec demo.cast
      Records a shell session to a file

  asciinema rec --command \"python script.py\" demo.cast
      Records execution of a Python script

  asciinema rec --idle-time-limit 2 demo.cast
      Records with idle time capped at 2 seconds

  asciinema rec --capture-input --title \"API Demo\" demo.cast
      Records with keyboard input and sets a title

  asciinema rec --append demo.cast
      Continues recording to an existing file

  asciinema rec demo.txt
      Records as a plain-text log - output format inferred from the .txt extension"
    )]
    Record(Record),

    /// Stream a terminal session in real-time.
    ///
    /// Broadcasts a terminal session live via either the local HTTP server (for local/LAN viewing) or a remote asciinema server (for public sharing). Viewers can watch the session as it happens through a web interface.
    ///
    /// Press <ctrl+d> or type 'exit' to end the streaming session.
    /// Press <ctrl+\> to pause/resume capture of the session.
    ///
    /// During the session, the ASCIINEMA_SESSION environment variable is set to a unique session ID.
    #[clap(
        about = "Stream a terminal session",
        long_about,
        after_help = "\x1b[1;4mExamples\x1b[0m:

  asciinema stream --local
      Streams a shell session via the local HTTP server listening on an ephemeral port on 127.0.0.1

  asciinema stream --local 0.0.0.0:8080
      Streams via the local HTTP server listening on port 8080 on all network interfaces

  asciinema stream --remote
      Streams via an asciinema server for public viewing

  asciinema stream -l -r
      Streams both locally and remotely simultaneously

  asciinema stream -r --command \"ping asciinema.org\"
      Streams execution of the ping command

  asciinema stream -r <ID> -t \"Live coding\"
      Streams via a remote server, reusing the existing stream ID and setting the stream title"
    )]
    Stream(Stream),

    /// Record and stream a terminal session simultaneously.
    ///
    /// Combines the functionality of record and stream commands, allowing you to save a recording to a file while also broadcasting it live to viewers.
    ///
    /// Press <ctrl+d> or type 'exit' to end the session.
    /// Press <ctrl+\> to pause/resume capture of the session.
    ///
    /// During the session, the ASCIINEMA_SESSION environment variable is set to a unique session ID.
    #[clap(
        about = "Record and stream a terminal session",
        long_about,
        after_help = "\x1b[1;4mExamples\x1b[0m:

  asciinema session --output-file demo.cast --stream-local
      Records a shell session to a file and streams it via the local HTTP server listening on an ephemeral port on 127.0.0.1

  asciinema session -o demo.cast --stream-remote
      Records to a file and streams via an asciinema server for public viewing

  asciinema session --stream-local --stream-remote
      Streams both locally and remotely simultaneously, without saving to a file

  asciinema session -o demo.cast -l -r -t \"Live coding\"
      Records + streams locally + streams remotely, setting the title of the recording/stream

  asciinema session -o demo.cast --idle-time-limit 1.5
      Records to a file with idle time capped at 1.5 seconds

  asciinema session -o demo.cast -l 0.0.0.0:9000 -r <ID>
      Records + streams locally on port 9000 + streams remotely, reusing existing stream ID"
    )]
    Session(Session),

    /// Play back a recorded terminal session.
    ///
    /// Displays a previously recorded asciicast file in your terminal with various playback controls (see below). Supports local files and remote URLs.
    ///
    /// Press <ctrl+c> to interrupt the playback.
    /// Press <space> to pause/resume.
    /// Press '.' to step forward (while paused).
    /// Press ']' to skip to the next marker (while paused).
    #[clap(
        about = "Play back a terminal session",
        long_about,
        after_help = "\x1b[1;4mExamples\x1b[0m:

  asciinema play demo.cast
      Plays back a local recording file once

  asciinema play --speed 2.0 --loop demo.cast
      Plays back at double speed in a loop

  asciinema play --idle-time-limit 2 demo.cast
      Plays back with idle time capped at 2 seconds

  asciinema play https://asciinema.org/a/569727
      Plays back directly from a URL

  asciinema play --pause-on-markers demo.cast
      Plays back, pausing automatically at every marker"
    )]
    Play(Play),

    /// Upload a recording to an asciinema server.
    ///
    /// Takes a local asciicast file and uploads it to an asciinema server (either asciinema.org or a self-hosted server), returning a recording URL which can be shared publicly.
    #[clap(about = "Upload a recording to an asciinema server", long_about)]
    Upload(Upload),

    /// Authenticate with an asciinema server.
    ///
    /// Creates a user account link between your local CLI and an asciinema server account. Optional for uploading with the upload command, required for remote streaming with the stream and session commands.
    #[clap(
        about = "Authenticate this CLI with an asciinema server account",
        long_about
    )]
    Auth(Auth),

    /// Concatenate multiple recordings into one.
    ///
    /// Combines two or more asciicast files in sequence, adjusting timing so each recording plays immediately after the previous one ends. Useful for creating longer recordings from multiple shorter sessions.
    ///
    /// Note: in asciinema 2.x this command used to print raw terminal output for a given session
    /// file. If you're looking for this behavior then use `asciinema convert -f raw <FILE> -` instead.
    #[clap(
        about = "Concatenate multiple recordings",
        long_about,
        after_help = "\x1b[1;4mExamples\x1b[0m:

  asciinema cat demo1.cast demo2.cast demo3.cast > combined.cast
      Combines local recordings into one file

  asciinema cat https://asciinema.org/a/569727 part2.cast > combined.cast
      Combines a remote and a local recording into one file"
    )]
    Cat(Cat),

    /// Convert a recording to another format.
    ///
    /// Transform asciicast files between different formats (v1, v2, v3) or export to other formats like raw terminal output or plain text. Supports reading from files, URLs, or stdin and writing to files or stdout.
    #[clap(
        about = "Convert a recording to another format",
        long_about,
        after_help = "\x1b[1;4mExamples\x1b[0m:

  asciinema convert old.cast new.cast
      Converts a recording to the latest asciicast format (v3)

  asciinema convert demo.cast demo.txt
      Exports a recording as a plain-text log - output format inferred from the .txt extension

  asciinema convert --output-format raw demo.cast demo.txt
      Exports as raw terminal output

  asciinema convert -f txt demo.cast -
      Exports as plain text to stdout

  asciinema convert https://asciinema.org/a/569727 starwars.cast
      Downloads a remote recording and converts it to the latest asciicast format (v3)"
    )]
    Convert(Convert),

    /// List the frames of a recording.
    ///
    /// Replays an asciicast file through a virtual terminal and prints one line per frame. A frame is any event that affects the rendered screen: terminal output ("o" events), terminal resizes ("r" events), and markers ("m" events - listed for orientation even though they don't alter the screen). Input and other event types are not frames and are skipped.
    ///
    /// For each frame the following is shown: the frame number (starting at 1), the event time in seconds, the time elapsed since the previous frame, the number of screen cells whose rendered contents (character or attributes) changed with this frame, the frame type, and a detail column showing the new terminal size for resize frames and the label for marker frames.
    ///
    /// Frame numbers shown by this command can be passed to the cat-frames command.
    #[clap(
        about = "List the frames of a recording",
        long_about,
        after_help = "\x1b[1;4mExamples\x1b[0m:

  asciinema ls-frames demo.cast
      Lists frames as a table

  asciinema ls-frames --format csv demo.cast
      Lists frames in CSV format

  asciinema ls-frames -f json demo.cast
      Lists frames as a JSON array"
    )]
    LsFrames(LsFrames),

    /// Print rendered frames of a recording.
    ///
    /// Replays an asciicast file through a virtual terminal and prints the full screen contents of the selected frames. Frames are selected by frame number (--frame), as shown by the ls-frames command, or by time in seconds (--time). Both options can be repeated and combined - the union of all selections is printed in frame order.
    ///
    /// A single time selects the frame displayed at that moment, i.e. the last frame rendered at or before the given time. A time range A-B selects all frames rendered within it, extended inclusively at both ends: the frame displayed at time A (rendered at or before A) and the first frame rendered at or after B are included as well.
    ///
    /// The terminal output format prints each frame as a screen snapshot preceded by a header line, with colors and text attributes reproduced using ANSI escape sequences, unless --no-escapes is used. The json output format produces an array of frame objects, each including both a plain-text rendering ("text") and an escape-sequence-based rendering ("seq") of the screen lines.
    #[clap(
        about = "Print rendered frames of a recording",
        long_about,
        after_help = "\x1b[1;4mExamples\x1b[0m:

  asciinema cat-frames --frame 42 demo.cast
      Prints the screen as rendered by frame 42

  asciinema cat-frames --frame 1,40-45 --no-escapes demo.cast
      Prints frames 1 and 40 through 45 as plain text

  asciinema cat-frames --time 4.2 demo.cast
      Prints the screen displayed at time 4.2s

  asciinema cat-frames --time 1.5-3 --format json demo.cast
      Prints all frames displayed between 1.5s and 3s as JSON"
    )]
    CatFrames(CatFrames),
}

#[derive(Debug, Args)]
pub struct Record {
    /// Output file path. A .zst suffix enables zstd compression.
    pub file: String,

    /// Specify the format for the output file. The default is asciicast-v3. If the file path ends with .txt, the txt format will be selected automatically unless --output-format is explicitly specified.
    #[arg(
        short = 'f',
        long,
        value_enum,
        value_name = "FORMAT",
        help = "Output file format [default: asciicast-v3]",
        long_help
    )]
    pub output_format: Option<Format>,

    /// Specify the command to execute in the recording session. If not provided, asciinema will use your default shell from the $SHELL environment variable. This can be any command with arguments, for example: --command "python script.py" or --command "bash -l". Can also be set via the config file option session.command.
    #[arg(
        short,
        long,
        help = "Command to start in the session [default: $SHELL]",
        long_help
    )]
    pub command: Option<String>,

    /// Enable recording of keyboard input in addition to terminal output. When enabled, both what you type and what appears on the screen will be captured. Note that sensitive input like passwords will also be recorded when this option is enabled. Can also be set via the config file option session.capture_input.
    #[arg(
        long,
        short = 'I',
        alias = "stdin",
        help = "Enable input (keyboard) capture",
        long_help
    )]
    pub capture_input: bool,

    /// Specify which environment variables to capture and include in the recording metadata. This helps ensure the recording context is preserved, e.g., for auditing. Provide a comma-separated list of variable names, for example: --rec-env "USER,SHELL,TERM". If not specified, only the SHELL variable is captured by default. Can also be set via the config file option session.capture_env.
    #[arg(
        long,
        value_name = "VARS",
        help = "Comma-separated list of environment variables to capture [default: SHELL]",
        long_help
    )]
    pub capture_env: Option<String>,

    /// Append the new session to an existing recording file instead of creating a new one. This allows you to continue a previous recording session. The timing will be adjusted to maintain continuity from where the previous recording ended. Cannot be used together with --overwrite.
    #[arg(short, long, help = "Append to an existing recording file", long_help)]
    pub append: bool,

    /// Overwrite the output file if it already exists. By default, asciinema will refuse to overwrite existing files to prevent accidental data loss. Cannot be used together with --append.
    #[arg(
        long,
        conflicts_with = "append",
        help = "Overwrite the output file if it already exists",
        long_help
    )]
    pub overwrite: bool,

    /// Set a title that will be stored in the recording metadata. This title may be displayed by players and is useful for organizing and identifying recordings. For example: --title "Installing Podman on Ubuntu".
    #[arg(short, long, help = "Title of the recording", long_help)]
    pub title: Option<String>,

    /// Limit the maximum idle time recorded between terminal events to the specified number of seconds. Long pauses (such as when you step away from the terminal) will be capped at this duration in the recording, making playback more watchable. For example, --idle-time-limit 2.0 will ensure no pause longer than 2 seconds appears in the recording. Note that this option doesn't alter the original (captured) timing information and instead, embeds the idle time limit value in the metadata, which is interpreted by session players at playback time. This allows tweaking of the limit after recording. Can also be set via the config file option session.idle_time_limit.
    #[arg(
        short,
        long,
        value_name = "SECS",
        help = "Limit idle time to a given number of seconds",
        long_help
    )]
    pub idle_time_limit: Option<f64>,

    /// Record in headless mode without using the terminal for input/output. This is useful for automated or scripted recordings where you don't want asciinema to interfere with the current terminal session. The recorded command will still execute normally, but asciinema won't display its output in your terminal. Headless mode is enabled automatically when running in an environment where a terminal is not available.
    #[arg(
        long,
        help = "Headless mode - don't use the terminal for I/O",
        long_help
    )]
    pub headless: bool,

    /// Override the terminal window size used for the recording session. Specify dimensions as COLSxROWS (e.g., 80x24 for 80 columns by 24 rows). You can specify just columns (80x) or just rows (x24) to override only one dimension. This is useful for ensuring consistent recording dimensions regardless of your current terminal size.
    #[arg(long, value_name = "COLSxROWS", value_parser = parse_window_size, help = "Override session's terminal window size", long_help)]
    pub window_size: Option<(Option<u16>, Option<u16>)>,

    /// Make the asciinema command exit with the same status code as the recorded session. By default, asciinema exits with status 0 regardless of what happens in the recorded session. With this option, if the recorded command exits with a non-zero status, asciinema will also exit with the same status.
    #[arg(long, help = "Return the session's exit status", long_help)]
    pub return_: bool,

    /// Enable logging of internal events to a file at the specified path. Useful for debugging recording issues.
    #[arg(long, value_name = "PATH", help = "Log file path", long_help)]
    pub log_file: Option<PathBuf>,

    #[arg(long, hide = true)]
    pub cols: Option<u16>,

    #[arg(long, hide = true)]
    pub rows: Option<u16>,

    #[arg(long, hide = true)]
    pub raw: bool,
}

#[derive(Debug, Args)]
pub struct Play {
    /// The path to an asciicast file or HTTP(S) URL to play back. Can be a local file path, HTTP(S) URL for remote files, or '-' to read from standard input. Remote URLs allow playing recordings directly from the web without need for manual downloading. Supported formats include asciicast v1, v2, and v3, optionally compressed with zstd.
    pub file: String,

    /// Control the playback speed as a multiplier of the original timing. Values greater than 1.0 make playback faster, while values less than 1.0 make it slower. For example, --speed 2.0 plays at double speed, while --speed 0.5 plays at half speed. The default is 1.0 (original speed). Can also be set via the config file option playback.speed.
    #[arg(short, long, help = "Set playback speed", long_help)]
    pub speed: Option<f64>,

    /// Enable continuous looping of the recording. When the recording reaches the end, it will automatically restart from the beginning. This continues indefinitely until you interrupt playback with <ctrl+c>.
    #[arg(
        short,
        long,
        name = "loop",
        help = "Loop playback continuously",
        long_help
    )]
    pub loop_: bool,

    /// Limit the maximum idle time between events during playback to the specified number of seconds. Long pauses in the original recording (such as when the user stepped away) will be shortened to this duration, making playback more watchable. This overrides any idle time limit set in the recording itself or in your config file (playback.idle_time_limit).
    #[arg(
        short,
        long,
        value_name = "SECS",
        help = "Limit idle time to a given number of seconds",
        long_help
    )]
    pub idle_time_limit: Option<f64>,

    /// Automatically pause playback when encountering marker events. Markers are special events that can be added during recording to mark important points in a session. When this option is enabled, playback will pause at each marker, allowing you to control the flow of the demonstration. Use <space> to resume, '.' to step through events, or ']' to skip to the next marker.
    #[arg(short = 'm', long, help = "Automatically pause on markers", long_help)]
    pub pause_on_markers: bool,

    /// Automatically resize the terminal window to match the original recording dimensions. This option attempts to change your terminal size to match the size used when the recording was made, ensuring the output appears exactly as it was originally recorded. Note that this feature is only supported by some terminals and may not work in all environments.
    #[arg(
        short = 'r',
        long,
        help = "Auto-resize terminal to match original size",
        long_help
    )]
    pub resize: bool,
}

#[derive(Debug, Args)]
#[clap(group(ArgGroup::new("mode").args(&["local", "remote"]).multiple(true).required(true)))]
pub struct Stream {
    /// Start the local HTTP server to stream the session in real-time. Creates a web interface accessible via browser where viewers can watch the terminal session live. Optionally specify the bind address as IP:PORT (e.g., 0.0.0.0:8080 to allow external connections). If no address is provided, it listens on an automatically assigned ephemeral port on 127.0.0.1.
    #[arg(short, long, value_name = "IP:PORT", default_missing_value = DEFAULT_LISTEN_ADDR, num_args = 0..=1, help = "Stream via the local HTTP server", long_help)]
    pub local: Option<SocketAddr>,

    /// Stream the session to a remote asciinema server for public viewing. This allows sharing your session on the web with anyone who has the stream URL. You can provide either a stream ID of an existing stream configuration in your asciinema server account, or a direct WebSocket URL (ws:// or wss://) for custom servers. Omitting the value for this option lets the asciinema server allocate a new stream ID automatically.
    #[arg(short, long, value_name = "STREAM-ID|WS-URL", default_missing_value = "", num_args = 0..=1, value_parser = validate_forward_target, help = "Stream via remote asciinema server", long_help)]
    pub remote: Option<RelayTarget>,

    /// Specify the command to execute in the streaming session. If not provided, asciinema will use your default shell from the $SHELL environment variable. This can be any command with arguments, for example: --command "python script.py" or --command "bash -l". Can also be set via the config file option session.command.
    #[arg(
        short,
        long,
        help = "Command to start in the session [default: $SHELL]",
        long_help
    )]
    pub command: Option<String>,

    /// Enable recording of keyboard input in addition to terminal output. When enabled, both what you type and what appears on the screen will be captured. Note that sensitive input like passwords will also be recorded when this option is enabled. If the server has stream recording enabled then keyboard input will be included in the recording file created on the server side. Can also be set via the config file option session.capture_input.
    #[arg(long, short = 'I', help = "Enable input (keyboard) capture", long_help)]
    pub capture_input: bool,

    /// Specify which environment variables to capture and include in the stream metadata. Provide a comma-separated list of variable names, for example: --rec-env "USER,SHELL,TERM". If not specified, only the SHELL variable is captured by default. If the server has stream recording enabled then these environment variables will be included in the recording file created on the server side. Can also be set via the config file option session.capture_env.
    #[arg(
        long,
        value_name = "VARS",
        help = "Comma-separated list of environment variables to capture [default: SHELL]",
        long_help
    )]
    pub capture_env: Option<String>,

    /// Set a descriptive title for the streaming session. This title is displayed to viewers (when doing remote streaming with --remote). For example: --title "Building a REST API". If the server has stream recording enabled then the title will be included in the recording file created on the server side.
    #[arg(short, long, help = "Title of the session", long_help)]
    pub title: Option<String>,

    /// Set a description for the session. This description is displayed on the stream page (applies only to remote streaming with --remote) and can include formatting, links, and code blocks. Useful for providing context, instructions, or documentation for viewers.
    #[arg(
        long,
        help = "Description of the session (Markdown supported)",
        long_help
    )]
    pub description: Option<String>,

    /// Set the visibility level for the stream (applies only to remote streaming with --remote). Public streams appear in listings and on your profile page. Unlisted streams are accessible via a direct URL but don't appear in listings. Private streams are only accessible to the owner.
    #[arg(long, value_enum, help = "Visibility level", long_help)]
    pub visibility: Option<Visibility>,

    /// Specify the URL of a live audio stream (e.g., Icecast MP3/OGG) to synchronize with the terminal stream (applies only to remote streaming with --remote). When set, viewers can listen to audio commentary while watching the terminal. The audio URL is stored in the stream metadata and used by the player for synchronized playback. For example: --audio-url https://icecast.example.com/live.mp3
    #[arg(
        long,
        value_name = "URL",
        help = "Audio stream URL for synchronized playback",
        long_help
    )]
    pub audio_url: Option<String>,

    /// Stream in headless mode without using the terminal for input/output. This is useful for automated or scripted streaming where you don't want asciinema to interfere with the current terminal session. The streamed command will still execute normally and be visible to viewers, but won't be displayed in your local terminal. Headless mode is enabled automatically when running in an environment where a terminal is not available.
    #[arg(
        long,
        help = "Headless mode - don't use the terminal for I/O",
        long_help
    )]
    pub headless: bool,

    /// Override the terminal window size used for the streaming session. Specify dimensions as COLSxROWS (e.g., 80x24 for 80 columns by 24 rows). You can specify just columns (80x) or just rows (x24) to override only one dimension. This is useful for ensuring consistent streaming dimensions regardless of your current terminal size.
    #[arg(long, value_name = "COLSxROWS", value_parser = parse_window_size, help = "Override session's terminal window size", long_help)]
    pub window_size: Option<(Option<u16>, Option<u16>)>,

    /// Make the asciinema command exit with the same status code as the streamed session. By default, asciinema exits with status 0 regardless of what happens in the streamed session. With this option, if the streamed command exits with a non-zero status, asciinema will also exit with that same status.
    #[arg(long, help = "Return the session's exit status", long_help)]
    pub return_: bool,

    /// Enable logging of internal events to a file at the specified path. Useful for debugging streaming issues (connection errors, disconnections, etc.).
    #[arg(long, value_name = "PATH", help = "Log file path", long_help)]
    pub log_file: Option<PathBuf>,

    /// Specify a custom asciinema server URL for streaming to self-hosted servers. Use the base server URL (e.g., https://asciinema.example.com). Can also be set via the environment variable ASCIINEMA_SERVER_URL or the config file option server.url. If no server URL is configured via this option, environment variable, or config file, you will be prompted to choose one (defaulting to asciinema.org), which will be saved as a default.
    #[arg(long, value_name = "URL", help = "asciinema server URL", long_help)]
    pub server_url: Option<String>,
}

/// Visibility level for uploads and streams
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum Visibility {
    Public,
    Unlisted,
    Private,
}

#[derive(Debug, Args)]
#[clap(group(ArgGroup::new("mode").args(&["output_file", "stream_local", "stream_remote"]).multiple(true).required(true)))]
pub struct Session {
    /// Save the session to a file at the specified path. A .zst suffix enables zstd compression. Can be combined with local and remote streaming.
    #[arg(
        short,
        long,
        value_name = "PATH",
        help = "Save the session to a file",
        long_help
    )]
    pub output_file: Option<String>,

    /// Specify the format for the output file when saving is enabled with --output-file. The default is asciicast-v3. If the output file path ends with .txt, the txt format will be selected automatically unless this option is explicitly specified.
    #[arg(
        short = 'f',
        long,
        value_enum,
        value_name = "FORMAT",
        help = "Output file format [default: asciicast-v3]",
        long_help
    )]
    pub output_format: Option<Format>,

    /// Start the local HTTP server to stream the session in real-time. Creates a web interface accessible via browser where viewers can watch the terminal session live. Optionally specify the bind address as IP:PORT (e.g., 0.0.0.0:8080 to allow external connections). If no address is provided, it listends on an automatically assigned ephemeral port on 127.0.0.1. Can be combined with remote streaming and file output.
    #[arg(short = 'l', long, value_name = "IP:PORT", default_missing_value = DEFAULT_LISTEN_ADDR, num_args = 0..=1, help = "Stream via the local HTTP server", long_help)]
    pub stream_local: Option<SocketAddr>,

    /// Stream the session to a remote asciinema server for public viewing. This allows sharing your session on the web with anyone who has the stream URL. You can provide either a stream ID of an existing stream configuration in your asciinema server account, or a direct WebSocket URL (ws:// or wss://) for custom servers. Omitting the value for this option lets the asciinema server allocate a new stream ID automatically. Can be combined with local streaming and file output.
    #[arg(short = 'r', long, value_name = "STREAM-ID|WS-URL", default_missing_value = "", num_args = 0..=1, value_parser = validate_forward_target, help = "Stream via remote asciinema server", long_help)]
    pub stream_remote: Option<RelayTarget>,

    /// Specify the command to execute in the session. If not provided, asciinema will use your default shell from the $SHELL environment variable. This can be any command with arguments, for example: --command "python script.py" or --command "bash -l". Can also be set via the config file option session.command.
    #[arg(
        short,
        long,
        help = "Command to start in the session [default: $SHELL]",
        long_help
    )]
    pub command: Option<String>,

    /// Enable recording of keyboard input in addition to terminal output. When enabled, both what you type and what appears on the screen will be captured. Note that sensitive input like passwords will also be recorded when this option is enabled. If the server has stream recording enabled then keyboard input will be included in the recording file created on the server side. Can also be set via the config file option session.capture_input.
    #[arg(long, short = 'I', help = "Enable input (keyboard) capture", long_help)]
    pub capture_input: bool,

    /// Specify which environment variables to capture and include in the session metadata. Provide a comma-separated list of variable names, for example: --rec-env "USER,SHELL,TERM". If not specified, only the SHELL variable is captured by default. If the server has stream recording enabled then these environment variables will be included in the recording file created on the server side. Can also be set via the config file option session.capture_env.
    #[arg(
        long,
        value_name = "VARS",
        help = "Comma-separated list of environment variables to capture [default: SHELL]",
        long_help
    )]
    pub capture_env: Option<String>,

    /// Append the new session to an existing recording file instead of creating a new one. This allows you to continue a previous recording session. The timing will be adjusted to maintain continuity from where the previous recording ended. Cannot be used together with --overwrite. Only applies when --output-file is specified.
    #[arg(short, long, help = "Append to an existing recording file", long_help)]
    pub append: bool,

    /// Overwrite the output file if it already exists. By default, asciinema will refuse to overwrite existing files to prevent accidental data loss. Cannot be used together with --append. Only applies when --output-file is specified.
    #[arg(
        long,
        conflicts_with = "append",
        help = "Overwrite the output file if it already exists",
        long_help
    )]
    pub overwrite: bool,

    /// Set a title for the session that will be stored in the recording metadata and displayed to stream viewers (when doing remote streaming with --remote). For example: --title "Installing Podman on Ubuntu". If the server has stream recording enabled then the title will be included in the recording file created on the server side.
    #[arg(short, long, help = "Title of the session", long_help)]
    pub title: Option<String>,

    /// Set a description for the session. This description is displayed on the stream page (applies only to remote streaming with --stream-remote) and can include formatting, links, and code blocks. Useful for providing context, instructions, or documentation for viewers.
    #[arg(
        long,
        help = "Description of the session (Markdown supported)",
        long_help
    )]
    pub description: Option<String>,

    /// Set the visibility level for the stream (applies only to remote streaming with --stream-remote). Public streams appear in listings and on your profile page. Unlisted streams are accessible via a direct URL but don't appear in listings. Private streams are only accessible to the owner.
    #[arg(long, value_enum, help = "Stream visibility level", long_help)]
    pub visibility: Option<Visibility>,

    /// Specify the URL of a live audio stream (e.g., Icecast MP3/OGG) to synchronize with the terminal stream (applies only to remote streaming with --stream-remote). When set, viewers can listen to audio commentary while watching the terminal. The audio URL is stored in the stream metadata and used by the player for synchronized playback. For example: --audio-url https://icecast.example.com/live.mp3
    #[arg(
        long,
        value_name = "URL",
        help = "Audio stream URL for synchronized playback",
        long_help
    )]
    pub audio_url: Option<String>,

    /// Limit the maximum idle time recorded between terminal events to the specified number of seconds. Long pauses (such as when you step away from the terminal) will be capped at this duration in the recording, making playback more watchable. For example, --idle-time-limit 2.0 will ensure no pause longer than 2 seconds appears in the recording. Only applies when --output-file is specified. Note that this option doesn't alter the original (captured) timing information and instead, it embeds the idle time limit value in the metadata, which is interpreted by session players at playback time. This allows tweaking of the limit after recording. Can also be set via the config file option session.idle_time_limit.
    #[arg(
        short,
        long,
        value_name = "SECS",
        help = "Limit idle time to a given number of seconds",
        long_help
    )]
    pub idle_time_limit: Option<f64>,

    /// Run the session in headless mode without using the terminal for input/output. This is useful for automated or scripted sessions where you don't want asciinema to interfere with the current terminal session. The session command will still execute normally and be recorded/streamed, but won't be displayed in your local terminal. Headless mode is enabled automatically when running in an environment where a terminal is not available.
    #[arg(
        long,
        help = "Headless mode - don't use the terminal for I/O",
        long_help
    )]
    pub headless: bool,

    /// Override the terminal window size used for the session. Specify dimensions as COLSxROWS (e.g., 80x24 for 80 columns by 24 rows). You can specify just columns (80x) or just rows (x24) to override only one dimension. This is useful for ensuring consistent recording dimensions regardless of your current terminal size.
    #[arg(long, value_name = "COLSxROWS", value_parser = parse_window_size, help = "Override session's terminal window size", long_help)]
    pub window_size: Option<(Option<u16>, Option<u16>)>,

    /// Make the asciinema command exit with the same status code as the session command. By default, asciinema exits with status 0 regardless of what happens in the session. With this option, if the session command exits with a non-zero status, asciinema will also exit with that same status.
    #[arg(long, help = "Return the session's exit status", long_help)]
    pub return_: bool,

    /// Enable logging of internal events to a file at the specified path. Useful for debugging I/O issues (connection errors, disconnections, file write errors, etc.).
    #[arg(long, value_name = "PATH", help = "Log file path", long_help)]
    pub log_file: Option<PathBuf>,

    /// Specify a custom asciinema server URL for streaming to self-hosted servers. Use the base server URL (e.g., https://asciinema.example.com). Can also be set via environment variable ASCIINEMA_SERVER_URL or config file option server.url. If no server URL is configured via this option, environment variable, or config file, you will be prompted to choose one (defaulting to asciinema.org), which will be saved as a default.
    #[arg(long, value_name = "URL", help = "asciinema server URL", long_help)]
    pub server_url: Option<String>,

    #[arg(hide = true)]
    pub env: Vec<String>,
}

#[derive(Debug, Args)]
pub struct Cat {
    /// List of recording files to concatenate. Provide at least two file paths (local files or HTTP(S) URLs). The files will be combined in the order specified. All files must be in asciicast format and may optionally be compressed with zstd.
    #[arg(required = true, num_args = 2.., help = "Recording files to concatenate", long_help)]
    pub file: Vec<String>,
}

#[derive(Debug, Args)]
pub struct Convert {
    /// The source recording to convert. Can be a local file path, HTTP(S) URL for remote files, or '-' to read from standard input. Remote URLs allow converting recordings directly from the web without need for manual downloading. Supported input formats include asciicast v1, v2 and v3, optionally compressed with zstd.
    pub input: String,

    /// The output path for the converted recording. Can be a file path or '-' to write to standard output. A .zst suffix enables zstd compression.
    pub output: String,

    /// Specify the format for the converted recording. The default is asciicast-v3. If the output file path ends with .txt, the txt format will be selected automatically unless this option is explicitly specified.
    #[arg(
        short = 'f',
        long,
        value_enum,
        value_name = "FORMAT",
        help = "Output file format [default: asciicast-v3]",
        long_help
    )]
    pub output_format: Option<Format>,

    /// Overwrite the output file if it already exists. By default, asciinema will refuse to overwrite existing files to prevent accidental data loss. Has no effect when writing to stdout ('-').
    #[arg(
        long,
        help = "Overwrite the output file if it already exists",
        long_help
    )]
    pub overwrite: bool,
}

#[derive(Debug, Args)]
pub struct LsFrames {
    /// The path to an asciicast file or HTTP(S) URL to list frames of. Can be a local file path, HTTP(S) URL for remote files, or '-' to read from standard input. Supported formats include asciicast v1, v2, and v3, optionally compressed with zstd.
    pub file: String,

    /// Specify the output format. The table format is meant for humans, while csv and json are meant for further processing. In the json format resize frames carry the new terminal size in the "cols" and "rows" fields, and marker frames carry their label in the "label" field.
    #[arg(
        short,
        long,
        value_enum,
        value_name = "FORMAT",
        default_value_t = ListFormat::Table,
        help = "Output format",
        long_help
    )]
    pub format: ListFormat,
}

#[derive(Debug, Args)]
#[clap(group(ArgGroup::new("selection").args(&["frame", "time"]).multiple(true).required(true)))]
pub struct CatFrames {
    /// The path to an asciicast file or HTTP(S) URL to print frames of. Can be a local file path, HTTP(S) URL for remote files, or '-' to read from standard input. Supported formats include asciicast v1, v2, and v3, optionally compressed with zstd.
    pub file: String,

    /// Select frames by frame number, as shown by the ls-frames command (numbering starts at 1). Accepts a comma-separated list of frame numbers and inclusive ranges, for example: --frame 1 or --frame 1,4-7. Can be repeated and combined with --time.
    #[arg(
        long,
        value_name = "FRAMES",
        value_parser = parse_frame_spec,
        help = "Frames to print, e.g. \"1\" or \"1,4-7\"",
        long_help
    )]
    pub frame: Vec<FrameSpec>,

    /// Select frames by time in seconds. Accepts a comma-separated list of times and time ranges, for example: --time 4.2 or --time 0.5-0.9. A single time selects the frame displayed at that moment - the last frame rendered at or before the given time. A time range selects all frames rendered within it, plus the frame displayed at the start of the range and the first frame rendered at or after its end. Can be repeated and combined with --frame.
    #[arg(
        long,
        value_name = "TIMES",
        value_parser = parse_time_spec,
        help = "Times of frames to print, e.g. \"4.2\" or \"0.5-0.9\"",
        long_help
    )]
    pub time: Vec<TimeSpec>,

    /// Specify the output format. The terminal format prints each frame as a screen snapshot preceded by a header line. The json format produces an array of frame objects, each including both a plain-text rendering ("text") and an escape-sequence-based rendering ("seq") of the screen lines.
    #[arg(
        short,
        long,
        value_enum,
        value_name = "FORMAT",
        default_value_t = CatFramesFormat::Terminal,
        help = "Output format",
        long_help
    )]
    pub format: CatFramesFormat,

    /// Render frames as plain text, without ANSI escape sequences for colors and text attributes. Applies to the terminal output format only - the json format always includes both renderings.
    #[arg(
        long,
        help = "Render frames as plain text, without escape sequences",
        long_help
    )]
    pub no_escapes: bool,
}

#[derive(Debug, Args)]
pub struct Upload {
    /// The path to the asciicast recording file to upload, in a supported asciicast format (v1, v2, or v3), optionally compressed with zstd.
    pub file: String,

    /// Set a title for the recording that will be stored in the recording metadata and displayed to viewers. For example: --title "Installing Podman on Ubuntu". This option takes precedence over the "title" field from the recording file itself.
    #[arg(short, long, help = "Title of the recording", long_help)]
    pub title: Option<String>,

    /// Set a description for the recording. This description is displayed on the recording page and can include formatting, links, and code blocks. Useful for providing context, instructions, or documentation for viewers.
    #[arg(
        long,
        help = "Description of the recording (Markdown supported)",
        long_help
    )]
    pub description: Option<String>,

    /// Set the visibility level for the recording. Public recordings appear in listings, search results and on your profile page. Unlisted recordings are accessible via a direct URL but don't appear in listings. Private recordings are only accessible to the owner.
    #[arg(long, value_enum, help = "Recording visibility level", long_help)]
    pub visibility: Option<Visibility>,

    /// Specify the URL of an audio file (e.g., MP3/OGG) to synchronize with the terminal playback. When set, viewers can listen to audio commentary while watching the terminal. The audio URL is stored in the recording metadata and used by the player for synchronized playback. For example: --audio-url https://example.com/commentary.mp3
    #[arg(
        long,
        value_name = "URL",
        help = "Audio URL for synchronized playback",
        long_help
    )]
    pub audio_url: Option<String>,

    /// Specify a custom asciinema server URL for uploading to self-hosted servers. Use the base server URL (e.g., https://asciinema.example.com). Can also be set via environment variable ASCIINEMA_SERVER_URL or config file option server.url. If no server URL is configured via this option, environment variable, or config file, you will be prompted to choose one (defaulting to asciinema.org), which will be saved as a default.
    #[arg(long, value_name = "URL", help = "asciinema server URL", long_help)]
    pub server_url: Option<String>,
}

#[derive(Debug, Args)]
pub struct Auth {
    /// Specify a custom asciinema server URL for authenticating with self-hosted servers. Use the base server URL (e.g., https://asciinema.example.com). Can also be set via environment variable ASCIINEMA_SERVER_URL or config file option server.url. If no server URL is configured via this option, environment variable, or config file, you will be prompted to choose one (defaulting to asciinema.org), which will be saved as a default.
    #[arg(long, value_name = "URL", help = "asciinema server URL", long_help)]
    pub server_url: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, ValueEnum)]
pub enum Format {
    /// Full-featured session format, with timing and metadata (current generation) - https://docs.asciinema.org/manual/asciicast/v3/
    AsciicastV3,
    /// Full-featured session format, with timing and metadata (previous generation) - https://docs.asciinema.org/manual/asciicast/v2/
    AsciicastV2,
    /// Raw terminal output, including control sequences, without timing and metadata
    Raw,
    /// Plain text without colors or control sequences, human-readable
    Txt,
}

#[derive(Clone, Copy, Debug, PartialEq, ValueEnum)]
pub enum ListFormat {
    /// Human-readable table with aligned columns
    Table,
    /// Comma-separated values, with a header row
    Csv,
    /// JSON array of frame objects
    Json,
}

#[derive(Clone, Copy, Debug, PartialEq, ValueEnum)]
pub enum CatFramesFormat {
    /// Screen snapshots, each preceded by a header line
    Terminal,
    /// JSON array of frame objects
    Json,
}

/// A list of inclusive frame number ranges, e.g. "1,4-7".
#[derive(Debug, Clone)]
pub struct FrameSpec(#[allow(dead_code)] pub Vec<(usize, usize)>);

/// A list of time points and time ranges, e.g. "4.2" or "0.5-0.9".
#[derive(Debug, Clone)]
pub struct TimeSpec(#[allow(dead_code)] pub Vec<TimeSelector>);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimeSelector {
    Point(f64),
    Range(f64, f64),
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum RelayTarget {
    StreamId(String),
    WsProducerUrl(url::Url),
}

fn parse_frame_spec(s: &str) -> Result<FrameSpec, String> {
    let mut ranges = Vec::new();

    for part in s.split(',') {
        let part = part.trim();

        let (start, end) = match part.split_once('-') {
            Some((start, end)) => (start, end),
            None => (part, part),
        };

        let start: usize = start
            .trim()
            .parse()
            .map_err(|_| format!("invalid frame number: {part}"))?;

        let end: usize = end
            .trim()
            .parse()
            .map_err(|_| format!("invalid frame number: {part}"))?;

        if start == 0 || end == 0 {
            return Err("frame numbers start at 1".to_owned());
        }

        if end < start {
            return Err(format!("invalid frame range: {part}"));
        }

        ranges.push((start, end));
    }

    Ok(FrameSpec(ranges))
}

fn parse_time_spec(s: &str) -> Result<TimeSpec, String> {
    let mut selectors = Vec::new();

    for part in s.split(',') {
        let part = part.trim();

        match part.split_once('-') {
            Some((start, end)) => {
                let start = parse_seconds(start)?;
                let end = parse_seconds(end)?;

                if end < start {
                    return Err(format!("invalid time range: {part}"));
                }

                selectors.push(TimeSelector::Range(start, end));
            }

            None => {
                selectors.push(TimeSelector::Point(parse_seconds(part)?));
            }
        }
    }

    Ok(TimeSpec(selectors))
}

fn parse_seconds(s: &str) -> Result<f64, String> {
    let time: f64 = s.trim().parse().map_err(|_| format!("invalid time: {s}"))?;

    if !time.is_finite() || time < 0.0 {
        return Err(format!("invalid time: {s}"));
    }

    Ok(time)
}

fn parse_window_size(s: &str) -> Result<(Option<u16>, Option<u16>), String> {
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

#[cfg(test)]
mod tests {
    use super::{parse_frame_spec, parse_time_spec, TimeSelector};

    #[test]
    fn frame_spec() {
        assert_eq!(parse_frame_spec("1").unwrap().0, vec![(1, 1)]);
        assert_eq!(parse_frame_spec("4-7").unwrap().0, vec![(4, 7)]);

        assert_eq!(
            parse_frame_spec("1,4-7,9").unwrap().0,
            vec![(1, 1), (4, 7), (9, 9)]
        );

        assert!(parse_frame_spec("").is_err());
        assert!(parse_frame_spec("0").is_err());
        assert!(parse_frame_spec("7-4").is_err());
        assert!(parse_frame_spec("1,x").is_err());
        assert!(parse_frame_spec("1-2-3").is_err());
    }

    #[test]
    fn time_spec() {
        assert_eq!(
            parse_time_spec("4.2").unwrap().0,
            vec![TimeSelector::Point(4.2)]
        );

        assert_eq!(
            parse_time_spec("0.5-0.9").unwrap().0,
            vec![TimeSelector::Range(0.5, 0.9)]
        );

        assert_eq!(
            parse_time_spec("1,2-3").unwrap().0,
            vec![TimeSelector::Point(1.0), TimeSelector::Range(2.0, 3.0)]
        );

        assert!(parse_time_spec("").is_err());
        assert!(parse_time_spec("x").is_err());
        assert!(parse_time_spec("3-2").is_err());
        assert!(parse_time_spec("-1").is_err());
        assert!(parse_time_spec("nan").is_err());
    }
}
