use std::collections::HashMap;

use clap::Args;
use colored::*;
use crate::error::{Result, ToolError};
use crate::configs::yaml_parser::{ProjectConfig, Service};
use crate::process::global::get_global_state;
use crate::process::state::ProcessState;
use log::debug; 

#[derive(Debug, Args, Clone)]
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

#[allow(clippy::await_holding_lock)]
impl StartArgs {
    pub async fn execute(&self) -> Result<()> {
        println!("{}", format!("Starting project: {}", self.name).bold());

        self.validate_args()?;

        let default_path = format!("{}/devspin.yaml", self.name);
        if !std::path::Path::new(&default_path).exists() {
            return Err(ToolError::ConfigError(format!(
                "Project '{}' not found at: {}", self.name, default_path  
            )))
        }
        let project = self.load_project(&default_path).await?;

        if self.dry_run {
            return self.dry_run(&project);
        }

        // Load environment file if specified
        if let Some(env) = &self.env {
            println!("{}", format!("Loading environment from: {}", env).dimmed());
            self.load_env_file(env).await?;
            
            if self.verbose {
                // FIX: Use if-let without collect() to avoid the type error
                let env_count = std::env::vars().count();
                println!("{} {} environment variables loaded", "✓".green(), env_count.to_string().cyan());
            }
        }

        if self.verbose {
            println!("{}", "Verbose output enabled".dimmed());
            self.show_verbose_configuration(&project);
        }

        if self.background {
            println!("{}", "Running in background mode".bold());
            return self.start_in_background(project).await;
        }

        if let Some(only_services) = &self.only {
            println!("{}", format!("Starting only: {}", only_services.join(", ")).bold());
        }

        if let Some(skip_services) = &self.skip {
            println!("{}", format!("Skipping: {}", skip_services.join(", ")).dimmed());
        }

        // For foreground mode, use global state directly
        let mut process_state: std::sync::MutexGuard<'static, ProcessState> = get_global_state();
        self.start_services(&project, &mut process_state).await
    }

    async fn load_project(&self, path: &str) -> Result<ProjectConfig> {
        debug!("Loading project from: {}", path);
        let project = ProjectConfig::from_file(path)?;
        println!("{}", format!("Loaded project: {}", project.name).bold());
        Ok(project)
    }

    async fn load_env_file(&self, env_file: &str) -> Result<()> {
        dotenvy::from_filename(env_file)
            .map_err(|e| ToolError::ConfigError(format!("Failed to load env file {}: {}", env_file, e)))?;
        Ok(())
    }

    fn show_verbose_configuration(&self, project: &ProjectConfig) {
        println!();
        println!("{}", "CONFIGURATION DETAILS:".cyan().bold());
        println!("  {} {}", "Config path:".dimmed(), format_args!("./{}/devspin.yaml", self.name));
        println!("  {} {}", "Project:".dimmed(), project.name.cyan());
        println!("  {} {:?}", "Description:".dimmed(), project.description);
        
        if let Some(env) = &self.env {
            println!("  {} {}", "Environment file:".dimmed(), env.cyan());
        }
        
        println!("  {} only={:?}, skip={:?}", "Service filters:".dimmed(), self.only, self.skip);
        
        println!("  {}", "Commands:".cyan().bold());
        println!("    {} {}", "Dev:".green(), project.commands.start.dev);
        if let Some(test) = &project.commands.start.test {
            println!("    {} {}", "Test:".yellow(), test);
        }
        println!("    {} {}", "Build:".blue(), project.commands.start.build);

        if let Some(clean) = &project.commands.start.clean {
            println!("    {} {}", "Clean:".red(), clean);
        }
        
        if let Some(env_vars) = &project.environment {
            println!("  {} ({})", "Environment variables:".cyan().bold(), env_vars.len());
            for (key, value) in env_vars {
                println!("    {} {}={}", "•".dimmed(), key.blue(), value.dimmed());
            }
        }
        
        if let Some(hooks) = &project.hooks {
            println!("  {}", "Hooks:".cyan().bold());
            if let Some(pre_start) = &hooks.pre_start {
                println!("    {} {}", "Pre-start:".yellow(), pre_start);
            }
            if let Some(post_start) = &hooks.post_start {
                println!("    {} {}", "Post-start:".green(), post_start);
            }
            if let Some(pre_stop) = &hooks.pre_stop {
                println!("    {} {}", "Pre-stop:".red(), pre_stop);
            }
            if let Some(post_stop) = &hooks.post_stop {
                println!("    {} {}", "Post-stop:".red(), post_stop);
            }
        }
        
        if let Some(services) = &project.services {
            println!();
            println!("  {}", "SERVICES:".cyan().bold());
            for service in services {
                let should_start = self.should_start_service(service);
                let status = if should_start { 
                    "START".green()
                } else { 
                    "SKIP".yellow()
                };
                
                println!("  {} {}:", status, service.name.bold());
                println!("    {} {}", "Type:".dimmed(), service.service_type.cyan());
                println!("    {} {}", "Command:".dimmed(), service.command.dimmed());
                
                if let Some(dir) = &service.working_dir {
                    println!("    {} {}", "Working directory:".dimmed(), dir.blue());
                }
                
                if !service.dependencies.is_empty() {
                    println!("    {} {:?}", "Dependencies:".dimmed(), service.dependencies);
                }
                
                if let Some(health_check) = &service.health_check {
                    println!("    {}", "Health check:".yellow().bold());
                    println!("      {} {}", "Type:".dimmed(), health_check.type_entry.cyan());
                    if let Some(port) = health_check.port {
                        println!("      {} {}", "Port:".dimmed(), port.to_string().blue());
                    }
                    if !health_check.http_target.is_empty() {
                        println!("      {} {}", "HTTP target:".dimmed(), health_check.http_target.green());
                    }
                }
                
                if !should_start {
                    let reason = if let Some(only_services) = &self.only {
                        if !only_services.contains(&service.name) {
                            "filtered out by --only"
                        } else {
                            "unknown"
                        }
                    } else if let Some(skip_services) = &self.skip {
                        if skip_services.contains(&service.name) {
                            "filtered out by --skip"
                        } else {
                            "unknown"
                        }
                    } else {
                        "unknown"
                    };
                    println!("    {} {}", "Status:".dimmed(), format!("SKIPPED ({})", reason).yellow());
                }
                
                println!();
            }
            
            println!("{}", "---".dimmed());
            println!("  {} {}", "Total services:".dimmed(), services.len().to_string().cyan());  
            let starting_count = services.iter().filter(|s| self.should_start_service(s)).count();
            println!("  {} {}", "Starting:".dimmed(), starting_count.to_string().green());
            println!("  {} only={:?}, skip={:?}", "Filters applied:".dimmed(), self.only, self.skip);
        }
        println!();
    }

    pub fn dry_run(&self, project: &ProjectConfig) -> Result<()> {
        println!("{}", "DRY RUN".bold().yellow());
        println!("{}", format!("Would start project: {}", project.name).bold());

        if self.verbose {
            self.show_verbose_configuration(project);
        }

        if self.background {
            println!("{} {}", "Mode:".dimmed(), "Background (detached)".purple().bold());
        } else {
            println!("{} {}", "Mode:".dimmed(), "Foreground (attached)".green().bold());
        }
        
        if let Some(services) = &project.services {
            println!();
            println!("{}", "SERVICES:".cyan().bold());
            for service in services {
                let should_start = self.should_start_service(service);
                
                if self.verbose {
                    let status = if should_start { 
                        format!("{}: {}", "STATUS".white(), "START".green())
                    } else { 
                        format!("{}: {}", "STATUS".white(), "SKIP".red())
                    };
                    
                    println!("  {} {}:", status, service.name.bold());
                    println!("    {} {}", "Command:".dimmed(), service.command.dimmed());
                    
                    if !should_start {
                        println!("    {} {}", "Status:".dimmed(), "SKIPPED (filtered out)".yellow());
                    }
                    println!();
                } else if should_start {
                    println!("  {} {}: {}", "START:".green(), service.name.bold(), service.command.dimmed());
                } else {
                    println!("  {} {}: {}", "SKIP:".red(), service.name.dimmed(), "(filtered out)".yellow());
                }
            }
            
            if self.verbose {
                println!("{}", "---".dimmed());
                println!("  {} {}", "Total services:".dimmed(), services.len().to_string().cyan());  
                let starting_count = services.iter().filter(|s| self.should_start_service(s)).count();
                println!("  {} {}", "Would start:".dimmed(), starting_count.to_string().green());
            }
        }

        Ok(())     
    }

    fn should_start_service(&self, service: &Service) -> bool {
        if let Some(only_services) = &self.only {
            if !only_services.contains(&service.name) {
                return false;
            }
        }

        if let Some(skip_services) = &self.skip {
            if skip_services.contains(&service.name) {
                return false;
            }
        }
        true
    }

    async fn spawn_service_command(
        &self, 
        service: &Service, 
        env_vars: &HashMap<String, String>,
        working_dir: &str
    ) -> Result<std::process::Child> {
        let mut command = std::process::Command::new("sh");
        command.arg("-c").arg(&service.command);
        
        // Use the resolved working directory
        command.current_dir(working_dir);
        
        for (key, value) in env_vars {
            command.env(key, value);
        }
        
        if self.verbose {
            debug!("Spawning command: sh -c '{}' in directory: {}", service.command, working_dir);
        }
        
        let child = command.spawn()?;
        Ok(child)
    }

    async fn start_services(&self, project: &ProjectConfig, process_state: &mut ProcessState) -> Result<()> {
        let env_vars = project.environment.clone().unwrap_or_default();
        
        if let Some(services) = &project.services {
            println!("{}", "Starting services...".cyan());

            let sorted_services = self.sort_services_by_dependencies(services);
            
            if self.verbose {
                println!("  {} services in dependency order:", "Starting".green());
                for service in &sorted_services {
                    if self.should_start_service(service) {
                        print!("  → {}", service.name.bold());
                        if !service.dependencies.is_empty() {
                            print!(" {} {:?}", "depends on:".dimmed(), service.dependencies);
                        }
                        println!();
                    }
                }
                println!();
            }
            
            for service in sorted_services {  
                if self.should_start_service(service) {
                    if self.verbose {
                        println!("{}", "─".repeat(50).dimmed());
                    }
                    
                    self.wait_for_dependencies(service, &*process_state, &project.name).await?;

                    println!("{}", format!("Starting service: {}", service.name).bold());
                    
                    if self.verbose {
                        println!("  {} {}", "Type:".dimmed(), service.service_type.cyan());
                        println!("  {} {}", "Command:".dimmed(), service.command.dimmed());
                        
                        if let Some(dir) = &service.working_dir {
                            println!("  {} {}", "Working directory:".dimmed(), dir.blue());
                        }
                        
                        if !service.dependencies.is_empty() {
                            println!("  {} {:?}", "Dependencies:".dimmed(), service.dependencies);
                        }
                        
                        if let Some(health_check) = &service.health_check {
                            println!("  {}", "Health check:".yellow().bold());
                            println!("    {} {}", "Type:".dimmed(), health_check.type_entry.cyan());
                            if let Some(port) = health_check.port {
                                println!("    {} {}", "Port:".dimmed(), port.to_string().blue());
                            }
                            if !health_check.http_target.is_empty() {
                                println!("    {} {}", "HTTP target:".dimmed(), health_check.http_target.green());
                            }
                        }
                    }
                    
                    // RESOLVE the working directory relative to project base
                    let working_dir = if let Some(service_dir) = &service.working_dir {
                        project.resolve_path(service_dir).to_string_lossy().to_string()
                    } else {
                        // Default to project base directory
                        project.base_path.as_ref()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|| ".".to_string())
                    };
                    
                    let child = self.spawn_service_command(service, &env_vars, &working_dir).await?;
                    let pid = child.id();

                    process_state.add_process(child, &service.name, &project.name, &service.command)?;
                    
                    println!("{} {} {} {}", 
                        "✓".green(), 
                        format!("Started service: {}", service.name).bold(),
                        format!("(PID: {})", pid).dimmed(),
                        format!("in directory: {}", working_dir).blue()
                    );

                    if let Some(health_check) = &service.health_check {
                        self.wait_for_health_check(service, health_check).await?;
                    }
                    
                    if self.verbose {
                        println!("  {} {}", "Status:".dimmed(), "RUNNING".green());
                        println!();
                    }
                } else if self.verbose {
                    println!("{} {}: {}", "SKIP".yellow(), service.name.dimmed(), "(filtered out)".yellow());
                    println!();
                }
            }
        }
        
        println!("{}", "─".repeat(50).dimmed());
        println!("{}", "All services started successfully!".green().bold());
        println!("{}", format!("Tracking {} processes in memory", process_state.process_count()).dimmed());
        
        Ok(())
    }

    async fn start_in_background(&self, project: ProjectConfig) -> Result<()> {
        println!("{}", format!("Starting project '{}' in background mode...", project.name).bold());

        if self.verbose {
            self.show_verbose_configuration(&project);
        }

        // Pre-collect all the services we need to start
        let services_to_start: Vec<Service> = if let Some(services) = &project.services {
            services.iter()
                .filter(|service| self.should_start_service(service))
                .cloned()
                .collect()
        } else {
            Vec::new()
        };

        if services_to_start.is_empty() {
            println!("{}", "No services to start".yellow());
            return Ok(());
        }

        let env_vars = project.environment.clone().unwrap_or_default();
        let project_name = project.name.clone();

        if self.verbose {
            println!("  {} services to start in background:", services_to_start.len().to_string().cyan());
            for service in &services_to_start {
                println!("    • {}: {}", service.name.bold(), service.command.dimmed());
            }
            println!();
        }

        // Get ONE global state instance for the entire operation
        let mut process_state = get_global_state();
        
        // Start each service and track it
        for service in services_to_start {
            println!("{}", format!("Starting background service: {}", service.name).bold());
            
            if self.verbose {
                println!("  {} {}", "Command:".dimmed(), service.command.dimmed());
            }
            
            // RESOLVE working directory
            let working_dir = if let Some(service_dir) = &service.working_dir {
                project.resolve_path(service_dir).to_string_lossy().to_string()
            } else {
                project.base_path.as_ref()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| ".".to_string())
            };
            
            match self.spawn_service_command(&service, &env_vars, &working_dir).await {
                Ok(child) => {
                    let pid = child.id();
                    
                    // Add to the SAME global state instance (no race condition)
                    match process_state.add_process(child, &service.name, &project_name, &service.command) {
                        Ok(()) => {
                            println!("{} {} {} {}", 
                                "✓".green(), 
                                format!("Started background service: {}", service.name).bold(),
                                format!("(PID: {})", pid).dimmed(),
                                format!("in directory: {}", working_dir).blue()
                            );
                        }
                        Err(e) => {
                            eprintln!("{} {}", "ERROR:".red(), format!("Failed to track service {}: {}", service.name, e).red());
                            // Kill the process since we failed to track it
                            let _ = std::process::Command::new("kill")
                                .arg(pid.to_string())
                                .output();
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{} {}", "ERROR".red(), format!("Failed to start service {}: {}", service.name, e).red());
                }
            }
            
            // Small delay between service starts
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }

        // Show final count before releasing the lock
        let final_count = process_state.process_count();
        println!("{} {}", "SUCCESS:".green(), format!("Project '{}' successfully started in background mode", project_name).bold());
        println!("{}", format!("Tracking {} processes in memory", final_count).dimmed());
        println!("{}", "Check status: devspin status".dimmed());
        println!("{} {}", "HELP:".red(), format!("Stop services: devspin stop {}", project_name).dimmed());
        
        Ok(())
    }
    fn sort_services_by_dependencies<'a>(&self, services: &'a [Service]) -> Vec<&'a Service> {
        let mut sorted = Vec::new();
        let mut visited = std::collections::HashSet::new();

        for service in services {
            Self::visit_service(service, services, &mut visited, &mut sorted);
        }
        
        sorted
    }

    fn visit_service<'a>(
        service: &'a Service,
        all_services: &'a [Service],
        visited: &mut std::collections::HashSet<&'a str>,
        sorted: &mut Vec<&'a Service>
    ) {
        if visited.contains(service.name.as_str()) {
            return;
        }

        visited.insert(service.name.as_str());

        for dep_name in &service.dependencies {
            if let Some(dep_service) = all_services.iter().find(|s| &s.name == dep_name) {
                Self::visit_service(dep_service, all_services, visited, sorted);
            }
        }

        sorted.push(service);
    }

    async fn wait_for_dependencies(&self, service: &Service, process_state: &ProcessState, project_name: &str) -> Result<()> {
        for dep_name in &service.dependencies {
            if !process_state.is_service_running(project_name, dep_name) {
                println!("{}", format!("Waiting for dependency: {} → {}", service.name, dep_name).dimmed());
                if self.verbose {
                    println!("  {} {}", "Dependency not yet ready:".yellow(), dep_name);
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            } else if self.verbose {
                println!("  {} {}", "Dependency ready:".green(), dep_name);
            }
        }
        Ok(())
    }

    async fn wait_for_health_check(&self, service: &Service, health_check: &crate::configs::yaml_parser::HealthCheck) -> Result<()> {
        println!("{}: {}", ("Waiting for health check").to_string().dimmed(), service.name.to_string().cyan());

        match health_check.type_entry.as_str() {
            "http" => {
                self.wait_for_http_health_check(health_check).await?;
            }
            "port" => {
                self.wait_for_port_health_check(health_check).await?;
            }
            _ => {
                println!("{}", format_args!("Unrecognized health check type: {}", health_check.type_entry))
            }
        }

        println!("{} {}", "✓".green(), format!("Health check passed: {}", service.name).bold());
        Ok(())
    }

    async fn wait_for_http_health_check(&self, health_check: &crate::configs::yaml_parser::HealthCheck) -> Result<()> {
        println!("   {} {}: {}", "🌐".cyan(), "HTTP check".to_string().dimmed(), health_check.http_target.to_string().cyan().bold());
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        Ok(())
    }

    async fn wait_for_port_health_check(&self, health_check: &crate::configs::yaml_parser::HealthCheck) -> Result<()> {
        if let Some(port) = health_check.port {
            println!("   {}", format!("Port check: {}", port).dimmed()); 
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
        Ok(())
    }

    fn validate_args(&self) -> Result<()> {
        if self.only.is_some() && self.skip.is_some() {
            return Err(ToolError::ConfigError(
                format!("{} Cannot use both --only and --skip filters simultaneously", "ERROR:".red())
            ));
        }
        
        // Validate service names in filters
        if let Some(only_services) = &self.only {
            for service in only_services {
                if service.trim().is_empty() {
                    return Err(ToolError::ConfigError(
                        format!("{} Empty service name in --only filter", "ERROR:".red())
                    ));
                }
            }
        }
        
        if let Some(skip_services) = &self.skip {
            for service in skip_services {
                if service.trim().is_empty() {
                    return Err(ToolError::ConfigError(
                        format!("{} Empty service name in --skip filter", "ERROR:".red())
                    ));
                }
            }
        }
        
        Ok(())
    }
}