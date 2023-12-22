use anyhow::Result;
use clap::Args;

#[derive(Debug, Args)]
pub struct Cli {
    #[arg(required = true)]
    filename: Vec<String>,
}

impl Cli {
    pub fn run(self) -> Result<()> {
        todo!();
    }
}
