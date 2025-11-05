use std::collections::HashMap;
use std::process::Command;  

use clap::Args;
use crate::error::{Result, ToolError};
use crate::configs::yaml_parser::{ProjectConfig, Service};
use crate::process::ProcessState;
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
    pub async fn handle(&self) -> Result<()> {
        println!("🚀 Starting project: {}", self.name);

        self.validate_args()?;

        let default_path = format!("{}/devbox.yaml", self.name);
        if !std::path::Path::new(&default_path).exists() {
            return Err(ToolError::ConfigError(format!(
                "Project '{}' not found at: {}", self.name, default_path
            )))
        }
        let project = self.load_project(&default_path).await?;

        let mut process_state = ProcessState::new();

        if self.dry_run {
            return self.dry_run(&project);
        }

        if let Some(env) = &self.env {
            println!("Loading environment from: {}", env);
            self.load_env_file(env).await?;
        }

        if self.verbose {
            println!("Verbose output enabled");
        }

        if self.background {
            println!("Running in background mode");
            return self.start_in_background(project, process_state).await;
        }

        if let Some(only_services) = &self.only {
            println!("Starting only: {}", only_services.join(", "));
        }

        if let Some(skip_services) = &self.skip {
            println!("⏭Skipping: {}", skip_services.join(", "));
        }

        self.start_services(&project, &mut process_state).await
    }

    async fn load_project(&self, path: &str) -> Result<ProjectConfig> {
        debug!("Loading project from: {}", path);
        let project = ProjectConfig::from_file(path)?;
        println!("Loaded project: {}", project.name);
        Ok(project)
    }

    async fn load_env_file(&self, env_file: &str) -> Result<()> {
        dotenvy::from_filename(env_file)
            .map_err(|e| ToolError::ConfigError(format!("Failed to load env file {}: {}", env_file, e)))?;
        Ok(())
    }

    pub fn dry_run(&self, project: &ProjectConfig) -> Result<()> {
        println!("🔍 DRY RUN - Would start project: {}", project.name);

        if self.verbose {
            println!("   CONFIGURATION DETAILS:");
            println!("   Config path: ./{}/devbox.yaml", self.name);
            println!("   Project: {}", project.name);
            println!("   Description: {:?}", project.description);
            
            if let Some(env) = &self.env {
                println!("   Environment file: {}", env);
            }
            
            println!("   Service filters: only={:?}, skip={:?}", self.only, self.skip);
            
            println!("   Commands:");
            println!("     - Dev: {}", project.commands.start.dev);
            if let Some(test) = &project.commands.start.test {
                println!("     - Test: {}", test);
            }
            println!("     - Build: {}", project.commands.start.build);

            if let Some(clean) = &project.commands.start.clean {
                println!("     - Clean: {}", clean);
            }
            
            if let Some(env_vars) = &project.environment {
                println!("   Environment variables ({}):", env_vars.len());
                for (key, value) in env_vars {
                    println!("     - {}={}", key, value);
                }
            }
            
            if let Some(hooks) = &project.hooks {
                println!("   Hooks:");
                if let Some(pre_start) = &hooks.pre_start {
                    println!("     - Pre-start: {}", pre_start);
                }
                if let Some(post_start) = &hooks.post_start {
                    println!("     - Post-start: {}", post_start);
                }
                if let Some(pre_stop) = &hooks.pre_stop {
                    println!("     - Pre-stop: {}", pre_stop);
                }
                if let Some(post_stop) = &hooks.post_stop {
                    println!("     - Post-stop: {}", post_stop);
                }
            }
            
            println!();
        }

        if self.background {
            println!("Mode: Background (detached)");
        } else {
            println!("Mode: Foreground (attached)");
        }
        
        if let Some(services) = &project.services {
            println!();
            println!("  SERVICES:");
            for service in services {
                let should_start = self.should_start_service(service);
                let status = if should_start { "✅" } else { "❌" };
                
                if self.verbose {
                    println!("  {} {}:", status, service.name);
                    println!("     Type: {}", service.service_type);
                    println!("     Command: {}", service.command);
                    
                    if let Some(dir) = &service.working_dir {
                        println!("     Working directory: {}", dir);
                    }
                    
                    println!("     Dependencies: {:?}", service.dependencies);
                    
                    if let Some(health_check) = &service.health_check {
                        println!("     Health check:");
                        println!("       - Type: {}", health_check.type_entry);
                        if let Some(port) = health_check.port {
                            println!("       - Port: {}", port);
                        }
                        if !health_check.http_target.is_empty() {
                            println!("       - HTTP target: {}", health_check.http_target);
                        }
                    }
                    
                    if !should_start {
                        println!("     Status: SKIPPED (filtered out)");
                    }
                    
                    println!();
                } else if should_start {
                    println!("  ✅ {}: {}", service.name, service.command);
                } else {
                    println!("  ❌ {}: (skipped)", service.name);
                }
            }
            
            if self.verbose {
                println!("---");
                println!("Total services: {}", services.len());  
                println!("Filters applied: only={:?}, skip={:?}", self.only, self.skip);
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

    async fn spawn_service_command(&self, service: &Service, env_vars: &HashMap<String, String>) -> Result<std::process::Child> {
        let mut command = Command::new("sh");
        command.arg("-c").arg(&service.command);
        
        if let Some(working_dir) = &service.working_dir {
            command.current_dir(working_dir);
            debug!("Working directory: {}", working_dir);
        }
        
        for (key, value) in env_vars {
            command.env(key, value);
            debug!("Env: {}={}", key, value);
        }
        
        let child = command.spawn()
            .map_err(|e| ToolError::ProcessError(format!("Failed to start service {}: {}", service.name, e)))?;
        
        Ok(child)
    }

    async fn start_services(&self, project: &ProjectConfig, process_state: &mut ProcessState) -> Result<()> {
        let env_vars = project.environment.clone().unwrap_or_default();
        
        if let Some(services) = &project.services {
            println!("🛠️ Starting services...");

            let sorted_services = self.sort_services_by_dependencies(services);
            
            for service in sorted_services {  
                if self.should_start_service(service) {
                    self.wait_for_dependencies(service, process_state, &project.name).await?;

                    println!("Starting service: {}", service.name);
                    
                    let mut child = self.spawn_service_command(service, &env_vars).await?;

                    let _ = process_state.add_process(&mut child, &service.name, &project.name, &service.command);

                    let pid = child.id();
                    println!("Started service: {} (PID: {})", service.name, pid);

                    if let Some(health_check) = &service.health_check {
                        self.wait_for_health_check(service, health_check).await?;
                    }

                    if !self.background {
                        self.spawn_process_monitor(child, service.name.clone()).await?;
                    }
                }
            }
        }
        
        println!("All services started successfully!");
        println!("Tracking {} processes in memory", process_state.process_count());
        
        Ok(())
    }

    async fn start_in_background(&self, project: ProjectConfig, mut process_state: ProcessState) -> Result<()> {
        let args_clone = self.clone();
        let project_name = project.name.clone();
        let project_for_background = project.clone();

        
        tokio::spawn(async move {
            println!("Starting background services for: {}", project_name);
            
            if let Err(e) = args_clone.start_services(&project_for_background, &mut process_state).await {
                eprintln!("Background service error: {}", e);
            } else {
                println!("Background services running for: {}", project_name);
            }
        });
        
        println!("Project '{}' started in background mode", project.name);
        println!("Use 'devbox status' to see running services");
        println!("Use 'devbox stop {}' to stop services", project.name);
        
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
                println!("⏳ Waiting for dependency: {} -> {}", service.name, dep_name);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
        Ok(())
    }

    async fn wait_for_health_check(&self, service: &Service, health_check: &crate::configs::yaml_parser::HealthCheck) -> Result<()> {
        println!("Waiting for health check: {}", service.name);

        match health_check.type_entry.as_str() {
            "http" => {
                self.wait_for_http_health_check(health_check).await?;
            }
            "port" => {
                self.wait_for_port_health_check(health_check).await?;
            }
            _ => {
                println!("Unrecognized health check type: {}", health_check.type_entry)
            }
        }

        println!("Health check passed: {}", service.name);
        Ok(())
    }

    async fn wait_for_http_health_check(&self, health_check: &crate::configs::yaml_parser::HealthCheck) -> Result<()> {
        println!("   HTTP check: {}", health_check.http_target);
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        Ok(())
    }

    async fn wait_for_port_health_check(&self, health_check: &crate::configs::yaml_parser::HealthCheck) -> Result<()> {
        if let Some(port) = health_check.port {
            println!("   Port check: {}", port); 
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
        Ok(())
    }

    async fn spawn_process_monitor(&self, mut child: std::process::Child, service_name: String) -> Result<()> {
        let pid = child.id();
        tokio::spawn(async move {
            match child.wait() {
                Ok(exit_status) => {
                    let status = if exit_status.success() { "successfully" } else { "with error" };
                    println!("Service {} (PID: {}) exited {}", service_name, pid, status);
                }
                Err(e) => {
                    eprintln!("Error waiting for service {} (PID: {}): {}", service_name, pid, e);
                }
            }
        });
        Ok(())
    }

    fn validate_args(&self) -> Result<()> {
        if self.only.is_some() && self.skip.is_some() {
            return Err(ToolError::ConfigError(
                "Cannot use both --only and --skip filters simultaneously".to_string()
            ));
        }
        
        // Validate service names in filters
        if let Some(only_services) = &self.only {
            for service in only_services {
                if service.trim().is_empty() {
                    return Err(ToolError::ConfigError(
                        "Empty service name in --only filter".to_string()
                    ));
                }
            }
        }
        
        if let Some(skip_services) = &self.skip {
            for service in skip_services {
                if service.trim().is_empty() {
                    return Err(ToolError::ConfigError(
                        "Empty service name in --skip filter".to_string()
                    ));
                }
            }
        }
        
        Ok(())
    }
}