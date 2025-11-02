use clap::Args;
use crate::error::Result;
use log::{error, warn, info, debug, trace};

#[derive(Args)]
pub struct StartArgs {
    /// Project name
    pub name: String,
    
    /// Environment configuration file
    #[arg(long)]
    pub env: Option<String>,
    
    /// Show detailed output
    #[arg(long)]
    pub verbose: bool,

    /// Run in background
    #[arg(long)]
    pub background: bool,

    /// Show what would start without actually starting
    #[arg(long)]
    pub dry_run: bool,

    /// Only start specific services
    #[arg(long, value_delimiter = ',')]
    pub only: Option<Vec<String>>,

    /// Skip specific services
    #[arg(long, value_delimiter = ',')]
    pub skip: Option<Vec<String>>
}

impl StartArgs {
    pub fn handle(&self) -> Result<()> {
        info!("Starting project: {}", self.name);

        if let Some(env) = &self.env {
            info!("Starting with an env variable")
        }

        if self.verbose {
            info!("Verbose output enabled")
        }
        
        if self.background {
            info!("Running in background")
        }

        if self.dry_run {
            println!("DRY RUN - would start:");
            // implement logic
            return Ok(())
        }

        if let Some(only_services) = &self.only {
            info!("Starting only : {:?}", only_services)
        }

        if let Some(skip_services) = &self.skip {
            info!("Starting without: {:?}", skip_services)
        }

        Ok(())
    }
}