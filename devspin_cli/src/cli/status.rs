use clap::Args;
use colored::*;
use tokio::time::{interval, Duration};
use crossterm::{
    terminal::{Clear, ClearType},
    cursor::{MoveTo, Hide, Show},
    event::{self, Event, KeyCode},
    execute,
};
use std::io;
use crate::ProcessInfo;
use crate::error::Result;

#[derive(Debug, Args, Clone)]
pub struct StatusArgs {
    /// Show specific project only
    pub project_name: Option<String>,
    
    /// Follow mode (like tail -f)
    #[arg(short, long)]
    pub follow: bool,

    /// Show live logs output
    #[arg(short, long)]
    pub logs: bool,

    /// Refresh interval in seconds (for follow mode)
    #[arg(long, default_value = "2")]
    pub interval: u64,

    /// Show only services with errors
    #[arg(long)]
    pub errors: bool,

    /// Show resource usage (CPU, memory)
    #[arg(long)]
    pub resources: bool,

    /// Number of log lines to show per service
    #[arg(long, default_value = "10")]
    pub tail: usize,
}

impl StatusArgs {
    pub async fn execute(&self) -> Result<()> {
        if self.follow {
            self.follow_mode().await?;
        } else if self.logs {
            self.show_live_logs().await?;
        } else {
            self.show_current_state().await?;
        }
        Ok(())
    }

    async fn show_current_state(&self) -> Result<()> {
        println!("{}", "CURRENT SERVICE STATES".bright_cyan().bold());
        println!("{}", "=".repeat(80).cyan());

        let services = self.get_active_services().await?;
        
        if services.is_empty() {
            println!("{}", "No active services found".yellow());
            return Ok(());
        }

        for service in &services {
            self.print_service_state(service).await?;
            println!();
        }

        self.show_summary(&services).await?;
        Ok(())
    }

    async fn print_service_state(&self, service: &LiveServiceState) -> Result<()> {
        let status_badge = match service.health {
            ServiceHealth::Healthy => "[HEALTHY]".green().bold(),
            ServiceHealth::Unhealthy => "[UNHEALTHY]".red().bold(), 
            ServiceHealth::Starting => "[STARTING]".yellow().bold(),
            ServiceHealth::Unknown => "[UNKNOWN]".white().dimmed(),
        };

        // Service header
        println!("{} {} ({})", 
            status_badge, 
            service.name.bold().white(), 
            service.project.blue()
        );

        // Current status
        println!("  {}: {}", "Status".dimmed(), self.format_status(&service.status));
        
        // Process info
        println!("  {}: {} | {}", "Process".dimmed(), 
            format!("PID: {}", service.pid).yellow(),
            self.format_uptime(service.start_time).cyan()
        );

        // Last output/activity
        if let Some(last_output) = &service.last_output {
            println!("  {}: {}", "Last Output".dimmed(), last_output.truncate(60).dimmed());
        }

        // Resource usage if available
        if self.resources {
            if let Some(usage) = &service.resource_usage {
                println!("  {}: {:.1}% CPU, {} MB RAM", 
                    "Resources".dimmed(), 
                    usage.cpu_percent, 
                    usage.memory_mb
                );
                if let Some(ports) = &usage.listening_ports {
                    println!("  {}: {:?}", "Ports".dimmed(), ports);
                }
            }
        }

        // Errors if any
        if let Some(error) = &service.last_error {
            println!("  {}: {}", "Error".red().bold(), error.red());
        }

        // Recent log lines
        if self.logs && !service.recent_logs.is_empty() {
            println!("  {}:", "Recent Logs".dimmed());
            for log in service.recent_logs.iter().take(self.tail) {
                println!("    {}", log.dimmed());
            }
        }

        Ok(())
    }

    async fn follow_mode(&self) -> Result<()> {
        // Clear screen and setup
        execute!(io::stdout(), Clear(ClearType::All), MoveTo(0, 0), Hide)?;

        let mut refresh_interval = interval(Duration::from_secs(self.interval));
        
        loop {
            // Clear and redraw
            execute!(io::stdout(), Clear(ClearType::All), MoveTo(0, 0))?;
            
            println!("{}", "LIVE SERVICE MONITOR (Press 'q' to quit)".bright_cyan().bold());
            println!("  {} Refreshing every {} seconds", "Refresh".dimmed(), self.interval);
            println!("{}", "=".repeat(80).cyan());
            
            let services = self.get_active_services().await?;
            
            for service in &services {
                self.print_live_service_state(service).await?;
            }

            // Show summary in follow mode
            self.show_follow_summary(&services).await?;

            // Check for quit key
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.code == KeyCode::Char('q') {
                        break;
                    }
                }
            }

            refresh_interval.tick().await;
        }

        // Cleanup
        execute!(io::stdout(), Show)?;
        println!("\n{}", "Stopped live monitoring".green());
        
        Ok(())
    }

    async fn print_live_service_state(&self, service: &LiveServiceState) -> Result<()> {
        let status_indicator = match service.health {
            ServiceHealth::Healthy => "●".green(),
            ServiceHealth::Unhealthy => "●".red(),
            ServiceHealth::Starting => "●".yellow(),
            ServiceHealth::Unknown => "○".white().dimmed(),
        };

        let status_text = match service.health {
            ServiceHealth::Healthy => "HEALTHY".green(),
            ServiceHealth::Unhealthy => "ERROR".red(),
            ServiceHealth::Starting => "STARTING".yellow(),
            ServiceHealth::Unknown => "UNKNOWN".white().dimmed(),
        };

        println!("{} {:<20} {:<12} {:<8} {}",
            status_indicator,
            service.name.bold(),
            status_text,
            format!("PID:{}", service.pid).dimmed(),
            self.format_uptime(service.start_time).cyan()
        );

        // Show last line of output
        if let Some(last_line) = service.recent_logs.last() {
            println!("    {}", last_line.truncate(70).dimmed());
        }

        // Show resource usage in compact form
        if self.resources {
            if let Some(usage) = &service.resource_usage {
                println!("    CPU: {:.1}% | RAM: {} MB", usage.cpu_percent, usage.memory_mb);
            }
        }

        Ok(())
    }

    async fn show_follow_summary(&self, services: &[LiveServiceState]) -> Result<()> {
        println!();
        println!("{}", "-".repeat(40).dimmed());
        
        let healthy = services.iter().filter(|s| s.health == ServiceHealth::Healthy).count();
        let total = services.len();
        
        println!("  {}: {}/{} services healthy", 
            "Status".dimmed(), 
            healthy.to_string().green(), 
            total
        );
        println!("  {}: Press 'q' to exit", "Help".dimmed());
        
        Ok(())
    }

    async fn show_live_logs(&self) -> Result<()> {
        println!("{}", "SERVICE LOGS".bright_cyan().bold());
        println!("{}", "=".repeat(80).cyan());

        let services = self.get_active_services().await?;
        
        for service in &services {
            println!("{} {} ({})", 
                "LOG".cyan().bold(), 
                service.name.bold(), 
                service.project.blue()
            );
            println!("{}", "-".repeat(40).dimmed());

            if service.recent_logs.is_empty() {
                println!("  {}", "No recent logs".dimmed());
            } else {
                for log in service.recent_logs.iter().take(self.tail) {
                    println!("  {}", log.dimmed());
                }
            }
            println!();
        }

        Ok(())
    }

    async fn get_active_services(&self) -> Result<Vec<LiveServiceState>> {
        use crate::process::manager::ProcessManager;
        
        let mut active_services = Vec::new();
        
        // Get REAL services from ProcessManager
        let real_services = ProcessManager::get_running_services();
        
        for service in &real_services {
            // Convert ProcessInfo to LiveServiceState
            let live_service = self.convert_to_live_state(service);
            
            // Apply filters
            if let Some(project_filter) = &self.project_name {
                if &live_service.project != project_filter {
                    continue;
                }
            }
            
            if self.errors && live_service.health != ServiceHealth::Unhealthy {
                continue;
            }
            
            active_services.push(live_service);
        }
        
        Ok(active_services)
    }

    fn convert_to_live_state(&self, process_info: &ProcessInfo) -> LiveServiceState {
        // Determine health based on actual process state
        let health = match &process_info.status {
            crate::ProcessStatus::Running => {
                // You could add actual health checks here
                // For now, assume running processes are healthy
                ServiceHealth::Healthy
            }
            crate::ProcessStatus::Stopped => ServiceHealth::Unhealthy,
            crate::ProcessStatus::Error(_) => ServiceHealth::Unhealthy,
        };

        LiveServiceState {
            name: process_info.service_name.clone(),
            project: process_info.project_name.clone(),
            pid: process_info.pid,
            status: match &process_info.status {
                crate::ProcessStatus::Running => ServiceStatus::Running,
                crate::ProcessStatus::Stopped => ServiceStatus::Stopped,
                crate::ProcessStatus::Error(err) => ServiceStatus::Error(err.clone()),
            },
            health,
            start_time: process_info.start_time,
            last_output: None, // You'd need to capture this from process output
            last_error: match &process_info.status {
                crate::ProcessStatus::Error(err) => Some(err.clone()),
                _ => None,
            },
            recent_logs: Vec::new(), // You'd need to capture process stdout/stderr
            resource_usage: None, // You could implement this with system calls
        }
    }

    async fn show_summary(&self, services: &[LiveServiceState]) -> Result<()> {
        println!("{}", "SUMMARY".bright_green().bold());
        println!("{}", "-".repeat(40).dimmed());

        let healthy = services.iter().filter(|s| s.health == ServiceHealth::Healthy).count();
        let unhealthy = services.iter().filter(|s| s.health == ServiceHealth::Unhealthy).count();
        let starting = services.iter().filter(|s| s.health == ServiceHealth::Starting).count();
        let total = services.len();

        println!("  {}: {}", "Total Services".dimmed(), total);
        println!("  {}: {}", "Healthy".green(), healthy);
        println!("  {}: {}", "Unhealthy".red(), unhealthy);
        if starting > 0 {
            println!("  {}: {}", "Starting".yellow(), starting);
        }

        if unhealthy > 0 {
            println!();
            println!("{}", "UNHEALTHY SERVICES:".red().bold());
            for service in services.iter().filter(|s| s.health == ServiceHealth::Unhealthy) {
                println!("  • {}: {}", service.name, service.last_error.as_deref().unwrap_or("Unknown error"));
            }
        }

        Ok(())
    }

    fn format_status(&self, status: &ServiceStatus) -> ColoredString {
        match status {
            ServiceStatus::Running => "Running".green(),
            ServiceStatus::Starting => "Starting".yellow(),
            ServiceStatus::Stopped => "Stopped".red(),
            ServiceStatus::Error(_) => "Error".red().bold(),
            ServiceStatus::Restarting => "Restarting".yellow(),
        }
    }

    fn format_uptime(&self, start_time: std::time::SystemTime) -> String {
        match start_time.elapsed() {
            Ok(duration) => {
                let secs = duration.as_secs();
                if secs > 86400 {
                    format!("{}d {}h", secs / 86400, (secs % 86400) / 3600)
                } else if secs > 3600 {
                    format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
                } else if secs > 60 {
                    format!("{}m {}s", secs / 60, secs % 60)
                } else {
                    format!("{}s", secs)
                }
            }
            Err(_) => "unknown".to_string()
        }
    }
}

// Data structures for live service state
#[derive(Debug, Clone)]
pub struct LiveServiceState {
    pub name: String,
    pub project: String,
    pub pid: u32,
    pub status: ServiceStatus,
    pub health: ServiceHealth,
    pub start_time: std::time::SystemTime,
    pub last_output: Option<String>,
    pub last_error: Option<String>,
    pub recent_logs: Vec<String>,
    pub resource_usage: Option<ResourceUsage>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServiceStatus {
    Running,
    Starting,
    Stopped,
    Error(String),
    Restarting,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServiceHealth {
    Healthy,
    Unhealthy,
    Starting,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub cpu_percent: f32,
    pub memory_mb: u64,
    pub listening_ports: Option<Vec<u16>>,
}

// Extension trait for string truncation
trait Truncate {
    fn truncate(&self, max_chars: usize) -> String;
}

impl Truncate for String {
    fn truncate(&self, max_chars: usize) -> String {
        if self.chars().count() <= max_chars {
            self.clone()
        } else {
            self.chars().take(max_chars).collect::<String>() + "..."
        }
    }
}

impl Truncate for &str {
    fn truncate(&self, max_chars: usize) -> String {
        if self.chars().count() <= max_chars {
            self.to_string()
        } else {
            self.chars().take(max_chars).collect::<String>() + "..."
        }
    }
}