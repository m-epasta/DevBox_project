use clap::Args;
use colored::*;
use crate::error::{Result, ToolError};
use crate::process::manager::ProcessManager;
use crate::process::global::get_global_state;
use crate::ProcessInfo;
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[derive(Debug, Args, Clone)]
pub struct StopArgs {
    /// Project name to stop
    pub project_name: Option<String>,
    
    /// Stop specific services only
    #[arg(long, value_delimiter = ',')]
    pub only: Option<Vec<String>>,
    
    /// Skip specific services
    #[arg(long, value_delimiter = ',')]
    pub skip: Option<Vec<String>>,
    
    /// Force stop (SIGKILL instead of SIGTERM)
    #[arg(short, long)]
    pub force: bool,
    
    /// Stop all running projects
    #[arg(long)]
    pub all: bool,
    
    /// Timeout in seconds for graceful shutdown
    #[arg(long, default_value = "30")]
    pub timeout: u64,
    
    /// Show detailed output
    #[arg(long)]
    pub verbose: bool,
    
    /// Dry run - show what would be stopped without actually stopping
    #[arg(long)]
    pub dry_run: bool,
}

impl StopArgs {
    pub async fn execute(&self) -> Result<()> {
        self.validate_args()?;
        
        if self.dry_run {
            return self.dry_run_execute().await;
        }
        
        if self.all {
            self.stop_all_projects().await
        } else if let Some(project_name) = &self.project_name {
            self.stop_single_project(project_name).await
        } else {
            Err(ToolError::ConfigError(
                "Must specify either a project name or --all".to_string()
            ))
        }
    }
    
    fn validate_args(&self) -> Result<()> {
        if self.all && self.project_name.is_some() {
            return Err(ToolError::ConfigError(
                "Cannot use both --all and project name".to_string()
            ));
        }
        
        if self.only.is_some() && self.skip.is_some() {
            return Err(ToolError::ConfigError(
                "Cannot use both --only and --skip".to_string()
            ));
        }
        
        Ok(())
    }
    
    async fn stop_all_projects(&self) -> Result<()> {
        println!("{}", "Stopping all projects...".bright_red().bold());
        
        let all_services = ProcessManager::get_running_services();
        let projects: std::collections::HashSet<_> = all_services
            .iter()
            .map(|s| s.project_name.clone())
            .collect();
            
        if projects.is_empty() {
            println!("{}", "No running projects found".yellow());
            return Ok(());
        }
        
        println!("{} {} projects", "Found".dimmed(), projects.len());
        
        for project in projects {
            println!();
            self.stop_single_project(&project).await?;
        }
        
        println!();
        println!("{}", "All projects stopped successfully".green().bold());
        Ok(())
    }
    
    async fn stop_single_project(&self, project_name: &str) -> Result<()> {
        println!("{} {}", "Stopping project:".bright_red().bold(), project_name.bold());
        
        let services = self.get_services_for_project(project_name);
        
        if services.is_empty() {
            println!("{}", "No running services found for this project".yellow());
            return Ok(());
        }
        
        let services_to_stop: Vec<_> = services
            .into_iter()
            .filter(|service| self.should_stop_service(service))
            .collect();
            
        if services_to_stop.is_empty() {
            println!("{}", "No services to stop (filtered by --only/--skip)".yellow());
            return Ok(());
        }
        
        if self.verbose {
            self.show_stop_plan(&services_to_stop);
        }
        
        // Stop services in reverse dependency order
        let sorted_services = self.sort_services_for_shutdown(&services_to_stop);
        
        self.stop_services_gracefully(&sorted_services).await?;
        
        println!("{} {}", "✓".green(), format!("Project '{}' stopped successfully", project_name).bold());
        Ok(())
    }
    
    async fn stop_services_gracefully(&self, services: &[ProcessInfo]) -> Result<()> {
        let total_services = services.len();
        let mut stopped_count = 0;
        
        for service in services {
            println!("{} {}", "Stopping:".dimmed(), service.service_name.bold());
            
            if self.verbose {
                println!("  {} {}", "PID:".dimmed(), service.pid);
                println!("  {} {}", "Command:".dimmed(), service.command.dimmed());
            }
            
            match self.stop_single_service(service).await {
                Ok(()) => {
                    stopped_count += 1;
                    println!("  {} {}", "✓".green(), "Stopped successfully".green());
                    
                    // Update process state
                    self.remove_process(service.pid);
                }
                Err(e) => {
                    if self.force {
                        println!("  {} {}", "!".yellow(), "Graceful stop failed, forcing...".yellow());
                        self.force_stop_service(service).await?;
                        stopped_count += 1;
                        self.remove_process(service.pid);
                    } else {
                        return Err(ToolError::ProcessError(format!(
                            "Failed to stop service {}: {}. Use --force to kill it",
                            service.service_name, e
                        )));
                    }
                }
            }
            
            println!("  {} {}/{} services stopped", 
                "Progress:".dimmed(), stopped_count, total_services
            );
            println!();
        }
        
        Ok(())
    }
    
    async fn stop_single_service(&self, service: &ProcessInfo) -> Result<()> {
        let start_time = Instant::now();
        let timeout = Duration::from_secs(self.timeout);
        
        // Send SIGTERM first (graceful shutdown)
        if self.verbose {
            println!("  {} Sending graceful shutdown signal...", "WAIT".dimmed());
        }
        
        self.stop_process(service.pid)?;
        
        // Wait for process to exit
        while start_time.elapsed() < timeout {
            if !self.is_process_running(service.pid) {
                return Ok(());
            }
            sleep(Duration::from_millis(100)).await;
        }
        
        // If we get here, the process didn't exit gracefully
        Err(ToolError::ProcessError(format!(
            "Service {} (PID: {}) did not stop within {} seconds",
            service.service_name, service.pid, self.timeout
        )))
    }
    
    async fn force_stop_service(&self, service: &ProcessInfo) -> Result<()> {
        if self.verbose {
            println!("  {} Sending SIGKILL...", "FORCE".dimmed());
        }
        
        self.kill_process(service.pid)?;
        
        // Give it a moment to die
        sleep(Duration::from_millis(500)).await;
        
        if self.is_process_running(service.pid) {
            return Err(ToolError::ProcessError(format!(
                "Failed to force stop service {} (PID: {})",
                service.service_name, service.pid
            )));
        }
        
        Ok(())
    }
    
    fn sort_services_for_shutdown(&self, services: &[ProcessInfo]) -> Vec<ProcessInfo> {
        // For shutdown, we want to stop services in reverse dependency order
        // So if A depends on B, stop A first, then B
        let mut sorted = services.to_vec();
        
        // Simple reversal - in a real implementation you'd want proper dependency analysis
        sorted.reverse();
        
        sorted
    }
    
    fn should_stop_service(&self, service: &ProcessInfo) -> bool {
        if let Some(only_services) = &self.only {
            if !only_services.contains(&service.service_name) {
                return false;
            }
        }
        
        if let Some(skip_services) = &self.skip {
            if skip_services.contains(&service.service_name) {
                return false;
            }
        }
        
        true
    }
    
    fn show_stop_plan(&self, services: &[ProcessInfo]) {
        println!("{}", "Stop Plan:".cyan().bold());
        println!("  {} {} services", "Total:".dimmed(), services.len());
        println!("  {} {}", "Mode:".dimmed(), if self.force { "Force stop".red().bold() } else { "Graceful stop".green() });
        println!("  {} {} seconds", "Timeout:".dimmed(), self.timeout);
        
        println!("  {}:", "Services to stop:".dimmed());
        for service in services {
            println!("    • {} (PID: {})", service.service_name, service.pid);
        }
        println!();
    }
    
    async fn dry_run_execute(&self) -> Result<()> {
        println!("{}", "DRY RUN - No services will be actually stopped".yellow().bold());
        println!();
        
        if self.all {
            let all_services = ProcessManager::get_running_services();
            let projects: std::collections::HashSet<_> = all_services
                .iter()
                .map(|s| s.project_name.clone())
                .collect();
                
            println!("{} {} projects would be stopped:", "Would stop:".dimmed(), projects.len());
            
            for project in projects {
                let services = self.get_services_for_project(&project);
                let services_to_stop: Vec<_> = services
                    .into_iter()
                    .filter(|service| self.should_stop_service(service))
                    .collect();
                    
                println!();
                println!("  {}:", project.bold());
                for service in services_to_stop {
                    let stop_type = if self.force { "FORCE STOP" } else { "Graceful stop" };
                    println!("    • {} (PID: {}) - {}", service.service_name, service.pid, stop_type);
                }
            }
        } else if let Some(project_name) = &self.project_name {
            let services = self.get_services_for_project(project_name);
            let services_to_stop: Vec<_> = services
                .into_iter()
                .filter(|service| self.should_stop_service(service))
                .collect();
                
            println!("{} {}", "Would stop project:".dimmed(), project_name.bold());
            println!("{} {} services", "Would stop:".dimmed(), services_to_stop.len());
            
            for service in services_to_stop {
                let stop_type = if self.force { "FORCE STOP".red() } else { "Graceful stop".green() };
                println!("  • {} (PID: {}) - {}", service.service_name, service.pid, stop_type);
            }
        }
        
        println!();
        println!("{}", "Add --verbose to see more details".dimmed());
        
        Ok(())
    }
    
    // Helper methods that work with your existing ProcessManager
    
    fn get_services_for_project(&self, project_name: &str) -> Vec<ProcessInfo> {
        ProcessManager::get_running_services()
            .into_iter()
            .filter(|service| service.project_name == project_name)
            .collect()
    }
    
    fn stop_process(&self, pid: u32) -> Result<()> {
        // Send SIGTERM
        let output = std::process::Command::new("kill")
            .arg(pid.to_string())
            .output()
            .map_err(|e| ToolError::ProcessError(format!("Failed to send SIGTERM to PID {}: {}", pid, e)))?;
            
        if !output.status.success() {
            return Err(ToolError::ProcessError(format!("kill command failed for PID {}", pid)));
        }
        
        Ok(())
    }
    
    fn kill_process(&self, pid: u32) -> Result<()> {
        // Send SIGKILL
        let output = std::process::Command::new("kill")
            .arg("-9")
            .arg(pid.to_string())
            .output()
            .map_err(|e| ToolError::ProcessError(format!("Failed to send SIGKILL to PID {}: {}", pid, e)))?;
            
        if !output.status.success() {
            return Err(ToolError::ProcessError(format!("kill -9 command failed for PID {}", pid)));
        }
        
        Ok(())
    }
    
    fn is_process_running(&self, pid: u32) -> bool {
        // Check if process exists by sending signal 0
        std::process::Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    
    fn remove_process(&self, pid: u32) {
        let mut state = get_global_state();
        let _ = state.remove_process(pid);
    }
}