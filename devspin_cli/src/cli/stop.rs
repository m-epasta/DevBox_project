use std::collections::HashMap;

use clap::Args;
use crate::error::{Result, ToolError};
use crate::configs::yaml_parser::{ProjectConfig, Service};
use crate::process::global::get_global_state;
use crate::process::state::ProcessState;
use log::debug; 

#[derive(Debug, Args, Clone)]
pub struct StopArgs {
    /// project name: it is optionnal because we can stop multiple projects
    pub name: Option<String>,

    /// stop all the services
    #[arg(long)]
    pub all: bool,

    /// stop all the projects
    #[arg(long)]
    pub all_projects: bool,

    /// stop specific projects
    #[arg(long, value_delimiter = ',')]
    pub projects: String,

    /// allow some services to keep running
    #[arg(long, value_delimiter = ',')]
    pub only: String,

    /// Skip specific services
    #[arg(long, value_delimiter = ',')]
    pub skip: Option<Vec<String>>,
    
    /// force all services to stop (un-safe)
    #[arg(long)]
    pub force: bool,
}

impl StopArgs {
    pub async fn execute(&self) -> Result<()> {
        println!("Stop devspin project");



        Ok(())
    }

}
