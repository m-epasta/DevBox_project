use std::collections::HashMap;
use clap::Args;
use colored::*;
use crate::ProcessInfo;
use crate::error::Result;
use crate::process::manager::ProcessManager;
use crate::process::state::ProcessStatus;

#[derive(Debug, Args, Clone)]
pub struct StatusArgs {
    /// Show specific project only
    pub project_name: Option<String>,
    
    /// Show status for specific file/directory
    #[arg(long, value_name = "FILE_OR_DIR", value_delimiter = ',')]
    pub files: Option<Vec<String>>,

    /// Show git-related status (branches, dirty files)
    #[arg(long)]
    pub git: bool,

    /// Show devbox configuration status
    #[arg(long)]
    pub devspin: bool,

    /// Show all projects (even empty ones)
    #[arg(long)]
    pub all: bool,

    /// Show detailed output
    #[arg(long)]
    pub verbose: bool,

    /// show JSON format for APIs / automated
    #[arg(long)]
    pub json: bool
}

impl StatusArgs {
    pub async fn execute(&self) -> Result<()> {
        println!("{}", "==== STATUS ====".bright_cyan().bold());

        self.show_status().await?;

        Ok(())
    }
    
    async fn show_status(&self) -> Result<()> {
        let services = self.get_filtered_services().await?;

        if self.json {
            self.show_json_status(&services)?;
        } else {
            self.show_human_status(&services).await?;
        }

        Ok(())
    }
    
    async fn get_filtered_services(&self) -> Result<Vec<ProcessInfo>> {
        let all_services = ProcessManager::get_running_services();
        
        let filtered = if let Some(project) = &self.project_name {
            all_services.into_iter()
                .filter(|service| service.project_name == *project)
                .collect()
        } else {
            all_services
        };
        
        Ok(filtered)
    }
    
    fn show_json_status(&self, services: &[ProcessInfo]) -> Result<()> {
        Ok(())
    }
    
    async fn show_human_status(&self, services: &[ProcessInfo]) -> Result<()> {
        if services.is_empty() {
            println!("{}", "No services running".yellow());
            return Ok(());
        }
        
        println!("{}", "RUNNING SERVICES:".bright_cyan().bold());
        println!("{}", "─────────────────────────────────────".cyan());
        
        let grouped = self.group_services_by_project(services.to_vec());
        
        for (project, services) in grouped {
            println!("{} {}", "PROJECT:".blue().bold(), project.blue().bold());
            for service in services {
                let duration = self.format_duration_since(service.start_time);
                let status_text = self.format_status_indicator(&service.status);
                println!("  {} {} (PID: {}) [{}]", status_text, service.service_name.white(), service.pid.to_string().yellow(), duration.cyan());
            }
            println!();
        }
        
        Ok(())
    }

    fn group_services_by_project(&self, services: Vec<ProcessInfo>) -> HashMap<String, Vec<ProcessInfo>> {
        let mut grouped = HashMap::new();
        
        for service in services {
            grouped.entry(service.project_name.clone())
                .or_insert_with(Vec::new)
                .push(service);
        }
        
        grouped
    }
    
    fn format_duration_since(&self, start_time: std::time::SystemTime) -> String {
        if let Ok(duration) = start_time.elapsed() {
            if duration.as_secs() > 60 {
                format!("{} min", duration.as_secs() / 60)
            } else {
                format!("{} sec", duration.as_secs())
            }
        } else {
            "unknown".to_string()
        }
    }

    fn format_status_indicator(&self, status: &ProcessStatus) -> ColoredString {
        match status {
            ProcessStatus::Running => "● RUNNING".green().bold(),
            ProcessStatus::Stopped => "● STOPPED".red().bold(),
            ProcessStatus::Error(_) => "● ERROR".red().bold(),
        }
    }
}