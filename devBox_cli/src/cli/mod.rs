use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "devbox")]
#[command(about = "Development environment manager")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start a development project
    Start(start::StartArgs),
    // ///Stop a running project
    // Stop(stop::StopArgs),
    // /// List all projects
    // List(list::ListArgs),
    // /// Show project status
    // Status(status::StatusArgs),
    // /// Initialize a new project
    // Init(init::InitArgs),
    // /// Show project logs
    // Logs(logs::LogsArgs),
    // /// Restart a project
    // Restart(restart::RestartArgs),
    // /// Manage project configuration
    // Config(config::ConfigArgs),
}

pub mod start;
pub mod stop;
pub mod list;
pub mod status;
pub mod init;
// pub mod logs;
// pub mod restart;
// pub mod config;