use anyhow::Error;
use clap::{Parser, Subcommand};
use run::{RunArgs, run};

pub mod run;

#[derive(Debug, Clone, Parser)]
pub struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Cmd {
    #[command(about = "Run the tests")]
    Run(RunArgs),
}

pub fn handle_args(args: &Cli) -> Result<(), Error> {
    match args.cmd {
        Cmd::Run(ref run_args) => {
            run(run_args)?;
        }
    }
    Ok(())
}
