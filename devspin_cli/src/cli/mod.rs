use clap::{Parser, Subcommand};
use crate::error::ToolError;

#[derive(Parser)]
#[command(name = "devspin")]
#[command(about = "Development environment manager")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start a development project
    Start(start::StartArgs),
    /// Stop a running project
    Stop(stop::StopArgs),
    /// List all projects
    // List(list::ListArgs),
    /// Show project status
    Status(status::StatusArgs),
    /// Initialize a new project
    Init(init::InitArgs),
    // /// Show project logs
    // Logs(logs::LogsArgs),
    // /// Restart a project
    // Restart(restart::RestartArgs),
    // /// Manage project configuration
    // Config(config::ConfigArgs),
    /// Show welcome message
    Welcome,
}

impl Cli {
    pub async fn execute(self) -> Result<(), ToolError> {
        match self.command {
            Commands::Start(args) => args.execute().await,
            Commands::Stop(args) => args.execute().await,
            Commands::Status(args) => args.execute().await,
            Commands::Init(args) => args.execute().await,
            Commands::Welcome => {
                welcome_message::welcome_message();
                Ok(())
            }
        }
    }
}

// Module declarations
pub mod start;
pub mod stop; 
pub mod list;
pub mod status;
pub mod init;
pub mod welcome_message;
// pub mod logs;
// pub mod restart;
// pub mod config;