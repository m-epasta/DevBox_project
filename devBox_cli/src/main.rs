use crate::error::Result;
mod error;
mod cli;
use crate::cli::{Cli, Commands, start};
use clap::Parser;
use log::{error, warn, info, debug, trace};


fn main() -> crate::error::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start(args) => args.handle()?,

    }
    Ok(())
}
