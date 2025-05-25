use clap::{Parser, Subcommand};
use run::RunArgs;

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
