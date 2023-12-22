use anyhow::Result;
use clap::Args;

#[derive(Debug, Args)]
pub struct Cli {
    filename: String,

    /// Limit idle time to given number of seconds
    #[arg(short, long, value_name = "SECS")]
    idle_time_limit: Option<f64>,

    /// Set playback speed
    #[arg(short, long)]
    speed: Option<f64>,

    /// Loop loop loop loop
    #[arg(short, long, name = "loop")]
    loop_: bool,

    /// Automatically pause on markers
    #[arg(short = 'm', long)]
    pause_on_markers: bool,
}

impl Cli {
    pub fn run(self) -> Result<()> {
        todo!();
    }
}
