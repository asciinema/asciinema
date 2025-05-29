mod alis;
mod api;
mod asciicast;
mod cli;
mod cmd;
mod config;
mod encoder;
mod file_writer;
mod forwarder;
mod hash;
mod html;
mod io;
mod leb128;
mod locale;
mod notifier;
mod player;
mod pty;
mod server;
mod session;
mod status;
mod stream;
mod tty;
mod util;

use clap::Parser;

use self::cli::{Cli, Commands, Session};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if cli.quiet {
        status::disable();
    }

    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    match cli.command {
        Commands::Rec(cmd) => {
            let cmd = Session {
                output_file: Some(cmd.output_path),
                rec_input: cmd.rec_input,
                append: cmd.append,
                output_format: cmd.output_format,
                overwrite: cmd.overwrite,
                command: cmd.command,
                rec_env: cmd.rec_env,
                title: cmd.title,
                idle_time_limit: cmd.idle_time_limit,
                headless: cmd.headless,
                window_size: cmd.window_size,
                stream_local: None,
                stream_remote: None,
                log_file: None,
                server_url: None,
            };

            cmd.run()
        }

        Commands::Stream(stream) => {
            let cmd = Session {
                output_file: None,
                rec_input: stream.rec_input,
                append: false,
                output_format: None,
                overwrite: false,
                command: stream.command,
                rec_env: stream.rec_env,
                title: stream.title,
                idle_time_limit: None,
                headless: stream.headless,
                window_size: stream.window_size,
                stream_local: stream.local,
                stream_remote: stream.remote,
                log_file: stream.log_file,
                server_url: stream.server_url,
            };

            cmd.run()
        }

        Commands::Session(cmd) => cmd.run(),
        Commands::Play(cmd) => cmd.run(),
        Commands::Cat(cmd) => cmd.run(),
        Commands::Convert(cmd) => cmd.run(),
        Commands::Upload(cmd) => cmd.run(),
        Commands::Auth(cmd) => cmd.run(),
    }
}
