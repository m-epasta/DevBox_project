use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use crate:: error::ToolError;
use log::{error, warn, info, debug, trace};

#[derive(Debug,Deserialize, Serialize)]
pub struct ProjectConfig {
    pub name: String,
    pub description: Option<String>,
    pub commands: Commands,
    pub environment: Option<HashMap<String, String>>,
    pub hooks: Option<Hooks>
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Commands {
    start: StartCommands
    // add other later
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StartCommands {
    pub dev: String,
    pub test: Option<String>,
    pub build: String,
    pub clean: Option<String>,

    pub services: Option<Services>
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Services {
    pub services: Vec<Service>
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Service {
    pub name: String,
    pub service_type: String,
    pub command: String,
    pub working_dir: Option<String>,
    pub health_check: Option<Health_check>,
    pub dependencies: Vec<String>
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Health_check {
    pub type_entry: String,
    pub port: Option<i16>,
    pub http_target: String
} 

#[derive(Debug, Deserialize, Serialize)]
pub struct Hooks {
    pub pre_start: Option<String>,
    pub post_start: Option<String>,
    pub pre_stop: Option<String>,
    pub post_stop: Option<String>
}

impl ProjectConfig {
    pub fn from_file(path: &str) -> Result<Self, ToolError> {
        let content = fs::read_to_string(path)?;
        info!("Loading project from {}", path);

        debug!("readed file");
        let config: ProjectConfig = serde_yaml::from_str(&content)
            .map_err(|e| ToolError::ParseError(e.to_string()))?;
        info!("successfully loaded project {}", config.name);
        Ok(config)
    }
}