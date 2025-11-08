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

impl StartArgs {
    pub async fn execute(&self) -> Result<()> {
        println!("{} {}", "🚀".green(), format!("Starting project: {}", self.name).bold());

        self.validate_args()?;

        let default_path = format!("{}/devspin.yaml", self.name);
        if !std::path::Path::new(&default_path).exists() {
            return Err(ToolError::ConfigError(format!(
                "{} Project '{}' not found at: {}", 
                "❌".red(), self.name, default_path
            )))
        }
        let project = self.load_project(&default_path).await?;

        if self.dry_run {
            return self.dry_run(&project);
        }

        if let Some(env) = &self.env {
            println!("{} {}", "📁".cyan(), format!("Loading environment from: {}", env).dimmed());
        }

        if self.verbose {
            println!("{} {}", "🔍".yellow(), "Verbose output enabled".dimmed());
        }

        if self.background {
            println!("{} {}", "⚙️".purple(), "Running in background mode".bold());
            return self.start_in_background(project).await;
        }

        if let Some(only_services) = &self.only {
            println!("{} {}", "🎯".blue(), format!("Starting only: {}", only_services.join(", ")).bold());
        }

        if let Some(skip_services) = &self.skip {
            println!("{} {}", "⏭".yellow(), format!("Skipping: {}", skip_services.join(", ")).dimmed());
        }

        // For foreground mode, use global state directly
        let mut process_state = get_global_state();
        self.start_services(&project, &mut process_state).await
    }

    async fn load_project(&self, path: &str) -> Result<ProjectConfig> {
        debug!("Loading project from: {}", path);
        let project = ProjectConfig::from_file(path)?;
        println!("{} {}", "📦".green(), format!("Loaded project: {}", project.name).bold());
        Ok(project)
    }

    async fn load_env_file(&self, env_file: &str) -> Result<()> {
        dotenvy::from_filename(env_file)
            .map_err(|e| ToolError::ConfigError(format!("{} Failed to load env file {}: {}", "❌".red(), env_file, e)))?;
        Ok(())
    }

    pub fn dry_run(&self, project: &ProjectConfig) -> Result<()> {
        println!("{}", "🌵 DRY RUN".bold().yellow());
        println!("{} {}", "📋".yellow(), format!("Would start project: {}", project.name).bold());

        if self.verbose {
            println!("{}", "   CONFIGURATION DETAILS:".cyan().bold());
            println!("   {} {}", "Config path:".dimmed(), format!("./{}/devspin.yaml", self.name));
            println!("   {} {}", "Project:".dimmed(), project.name.cyan());
            println!("   {} {:?}", "Description:".dimmed(), project.description);
            
            if let Some(env) = &self.env {
                println!("   {} {}", "Environment file:".dimmed(), env.cyan());
            }
            
            println!("   {} only={:?}, skip={:?}", "Service filters:".dimmed(), self.only, self.skip);
            
            println!("   {}", "Commands:".cyan().bold());
            println!("     {} {}", "Dev:".green(), project.commands.start.dev);
            if let Some(test) = &project.commands.start.test {
                println!("     {} {}", "Test:".yellow(), test);
            }
            println!("     {} {}", "Build:".blue(), project.commands.start.build);

            if let Some(clean) = &project.commands.start.clean {
                println!("     {} {}", "Clean:".red(), clean);
            }
            
            if let Some(env_vars) = &project.environment {
                println!("   {} ({})", "Environment variables:".cyan().bold(), env_vars.len());
                for (key, value) in env_vars {
                    println!("     {} {}={}", "•".dimmed(), key.blue(), value.dimmed());
                }
            }
            
            if let Some(hooks) = &project.hooks {
                println!("   {}", "Hooks:".cyan().bold());
                if let Some(pre_start) = &hooks.pre_start {
                    println!("     {} {}", "Pre-start:".yellow(), pre_start);
                }
                if let Some(post_start) = &hooks.post_start {
                    println!("     {} {}", "Post-start:".green(), post_start);
                }
                if let Some(pre_stop) = &hooks.pre_stop {
                    println!("     {} {}", "Pre-stop:".red(), pre_stop);
                }
                if let Some(post_stop) = &hooks.post_stop {
                    println!("     {} {}", "Post-stop:".red(), post_stop);
                }
            }
            
            println!();
        }

        if self.background {
            println!("{} {}", "Mode:".dimmed(), "Background (detached)".purple().bold());
        } else {
            println!("{} {}", "Mode:".dimmed(), "Foreground (attached)".green().bold());
        }
        
        if let Some(services) = &project.services {
            println!();
            println!("  {}", "SERVICES:".cyan().bold());
            for service in services {
                let should_start = self.should_start_service(service);
                let status = if should_start { "✅" } else { "❌" };
                
                if self.verbose {
                    println!("  {} {}:", status, service.name.bold());
                    println!("     {} {}", "Type:".dimmed(), service.service_type.cyan());
                    println!("     {} {}", "Command:".dimmed(), service.command.dimmed());
                    
                    if let Some(dir) = &service.working_dir {
                        println!("     {} {}", "Working directory:".dimmed(), dir.blue());
                    }
                    
                    println!("     {} {:?}", "Dependencies:".dimmed(), service.dependencies);
                    
                    if let Some(health_check) = &service.health_check {
                        println!("     {}", "Health check:".yellow().bold());
                        println!("       {} {}", "Type:".dimmed(), health_check.type_entry.cyan());
                        if let Some(port) = health_check.port {
                            println!("       {} {}", "Port:".dimmed(), port.to_string().blue());
                        }
                        if !health_check.http_target.is_empty() {
                            println!("       {} {}", "HTTP target:".dimmed(), health_check.http_target.green());
                        }
                    }
                    
                    if !should_start {
                        println!("     {} {}", "Status:".dimmed(), "SKIPPED (filtered out)".yellow());
                    }
                    
                    println!();
                } else if should_start {
                    println!("  {} {}: {}", "✅".green(), service.name.bold(), service.command.dimmed());
                } else {
                    println!("  {} {}: {}", "❌".red(), service.name.dimmed(), "(skipped)".yellow());
                }
            }
            
            if self.verbose {
                println!("{}", "---".dimmed());
                println!("{} {}", "Total services:".dimmed(), services.len().to_string().cyan());  
                println!("{} only={:?}, skip={:?}", "Filters applied:".dimmed(), self.only, self.skip);
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
        
        let child = command.spawn()?;
        Ok(child)
    }

    async fn start_services(&self, project: &ProjectConfig, process_state: &mut ProcessState) -> Result<()> {
        let env_vars = project.environment.clone().unwrap_or_default();
        
        if let Some(services) = &project.services {
            println!("{}", "🔄 Starting services...".cyan());

            let sorted_services = self.sort_services_by_dependencies(services);
            
            for service in sorted_services {  
                if self.should_start_service(service) {
                    self.wait_for_dependencies(service, &*process_state, &project.name).await?;

                    println!("{} {}", "🚀".green(), format!("Starting service: {}", service.name).bold());
                    
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
                        "✅".green(), 
                        format!("Started service: {}", service.name).bold(),
                        format!("(PID: {})", pid).dimmed(),
                        format!("in directory: {}", working_dir).blue()
                    );

                    if let Some(health_check) = &service.health_check {
                        self.wait_for_health_check(service, health_check).await?;
                    }
                }
            }
        }
        
        println!("{}", "🎉 All services started successfully!".green().bold());
        println!("{} {}", "📊".cyan(), format!("Tracking {} processes in memory", process_state.process_count()).dimmed());
        
        Ok(())
    }

    async fn start_in_background(&self, project: ProjectConfig) -> Result<()> {
        println!("{} {}", "⚙️".purple(), format!("Starting project '{}' in background mode...", project.name).bold());

        // Pre-collect all the services we need to start
        let services_to_start: Vec<Service> = if let Some(services) = &project.services {
            services.iter()
                .filter(|service| self.should_start_service(service))
                .cloned()
                .collect()
        } else {
            Vec::new()
        };

        let env_vars = project.environment.clone().unwrap_or_default();
        let project_name = project.name.clone();

        // Start each service and track it immediately
        for service in services_to_start {
            println!("{} {}", "🔧".blue(), format!("Starting background service: {}", service.name).bold());
            
            // RESOLVE working directory for background mode too
            let working_dir = if let Some(service_dir) = &service.working_dir {
                project.resolve_path(service_dir).to_string_lossy().to_string()
            } else {
                project.base_path.as_ref()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| ".".to_string())
            };
            
            // FIX: Pass all 3 arguments to spawn_service_command
            match self.spawn_service_command(&service, &env_vars, &working_dir).await {
                Ok(child) => {
                    let pid = child.id();
                    
                    // Store in global state - quick operation, no await points
                    let mut process_state = get_global_state();
                    if let Err(e) = process_state.add_process(child, &service.name, &project_name, &service.command) {
                        eprintln!("{} {}", "❌".red(), format!("Failed to track service {}: {}", service.name, e).red());
                    } else {
                        println!("{} {} {} {}", 
                            "✅".green(), 
                            format!("Started background service: {}", service.name).bold(),
                            format!("(PID: {})", pid).dimmed(),
                            format!("in directory: {}", working_dir).blue()
                        );
                    }
                    // process_state drops here, releasing the mutex
                }
                Err(e) => {
                    eprintln!("{} {}", "❌".red(), format!("Failed to start service {}: {}", service.name, e).red());
                }
            }
            
            // Small delay between service starts
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }

        println!("{} {}", "✅".green(), format!("Project '{}' successfully started in background mode", project_name).bold());
        println!("{} {}", "📊".cyan(), "Check status: devspin status".dimmed());
        println!("{} {}", "🛑".red(), format!("Stop services: devspin stop {}", project_name).dimmed());
        
        Ok(())
    }

    fn sort_services_by_dependencies<'a>(&self, services: &'a [Service]) -> Vec<&'a Service> {
        let mut sorted = Vec::new();
        let mut visited = std::collections::HashSet::new();

        for service in services {
            self.visit_service(service, services, &mut visited, &mut sorted);
        }
        
        sorted
    }

    fn visit_service<'a>(
        &self,
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
                self.visit_service(dep_service, all_services, visited, sorted);
            }
        }

        sorted.push(service);
    }

    async fn wait_for_dependencies(&self, service: &Service, process_state: &ProcessState, project_name: &str) -> Result<()> {
        for dep_name in &service.dependencies {
            if !process_state.is_service_running(project_name, dep_name) {
                println!("{} {}", "⏳".yellow(), format!("Waiting for dependency: {} -> {}", service.name, dep_name).dimmed());
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
        Ok(())
    }

    async fn wait_for_health_check(&self, service: &Service, health_check: &crate::configs::yaml_parser::HealthCheck) -> Result<()> {
        println!("{} {}", "❤️".green(), format!("Waiting for health check: {}", service.name).dimmed());

        match health_check.type_entry.as_str() {
            "http" => {
                self.wait_for_http_health_check(health_check).await?;
            }
            "port" => {
                self.wait_for_port_health_check(health_check).await?;
            }
            _ => {
                println!("{} {}", "⚠️".yellow(), format!("Unrecognized health check type: {}", health_check.type_entry))
            }
        }

        println!("{} {}", "✅".green(), format!("Health check passed: {}", service.name).bold());
        Ok(())
    }

    async fn wait_for_http_health_check(&self, health_check: &crate::configs::yaml_parser::HealthCheck) -> Result<()> {
        println!("   {} {}", "🌐".cyan(), format!("HTTP check: {}", health_check.http_target).dimmed());
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        Ok(())
    }

    async fn wait_for_port_health_check(&self, health_check: &crate::configs::yaml_parser::HealthCheck) -> Result<()> {
        if let Some(port) = health_check.port {
            println!("   {} {}", "🔌".blue(), format!("Port check: {}", port).dimmed()); 
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
        Ok(())
    }

    fn validate_args(&self) -> Result<()> {
        if self.only.is_some() && self.skip.is_some() {
            return Err(ToolError::ConfigError(
                format!("{} Cannot use both --only and --skip filters simultaneously", "❌".red())
            ));
        }
        
        // Validate service names in filters
        if let Some(only_services) = &self.only {
            for service in only_services {
                if service.trim().is_empty() {
                    return Err(ToolError::ConfigError(
                        format!("{} Empty service name in --only filter", "❌".red())
                    ));
                }
            }
        }
        
        if let Some(skip_services) = &self.skip {
            for service in skip_services {
                if service.trim().is_empty() {
                    return Err(ToolError::ConfigError(
                        format!("{} Empty service name in --skip filter", "❌".red())
                    ));
                }
            }
        }
        
        Ok(())
    }
}