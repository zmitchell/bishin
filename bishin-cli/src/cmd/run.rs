use anyhow::{Context, Error};
use std::path::PathBuf;

use bishin_config::Config;
use clap::Args;

#[derive(Debug, Clone, Args)]
pub struct RunArgs {
    /// The path to the config file (default is '$PWD/bishin.toml').
    #[arg(
        short = 'f',
        long = "config-file",
        value_name = "PATH",
        required = false
    )]
    pub config_file: Option<PathBuf>,
}

/// Run the test suite.
pub fn run(args: &RunArgs) -> Result<(), Error> {
    let config = Config::load(args.config_file.as_ref()).context("failed to load config file")?;
    eprintln!("config: {config:?}");
    Ok(())
}
