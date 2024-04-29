use clap::CommandFactory;
use clap::ValueEnum;
use clap_complete::{generate_to, Shell};
use clap_mangen::Man;
use std::env;
use std::fs::{create_dir_all, File};
use std::path::Path;
use std::path::PathBuf;

mod cli {
    include!("src/cli.rs");
}

const ENV_KEY: &str = "ASCIINEMA_GEN_DIR";

fn main() -> std::io::Result<()> {
    if let Some(dir) = env::var_os(ENV_KEY).or(env::var_os("OUT_DIR")) {
        let mut cmd = cli::Cli::command();
        let base_dir = PathBuf::from(dir);

        let man_dir = Path::join(&base_dir, "man");
        create_dir_all(&man_dir)?;
        let man_path = Path::join(&man_dir, "asciinema.1");
        Man::new(cmd.clone()).render(&mut File::create(man_path)?)?;

        let completion_dir = Path::join(&base_dir, "completion");
        create_dir_all(&completion_dir)?;

        for shell in Shell::value_variants() {
            generate_to(*shell, &mut cmd, "asciinema", &completion_dir)?;
        }
    }

    println!("cargo:rerun-if-env-changed={ENV_KEY}");

    Ok(())
}
