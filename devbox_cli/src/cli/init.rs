use clap::Args;
use std::io::{self, Write};
use crate::error::Result;
use std::process::Command;
use std::path::Path;
// TODO: test and refactor all the templates except nextjs
// Template data structures
#[derive(Debug, Clone)]
struct TemplateFile {
    path: String,
    content: &'static str,
}

#[derive(Debug, Clone)]
struct ServiceConfig {
    name: String,
    service_type: String,
    command: String,
    working_dir: String,
    health_check: HealthCheck,
    dependencies: Vec<String>,
}

#[derive(Debug, Clone)]
struct HealthCheck {
    type_entry: String,
    port: u16,
    http_target: String,
}

#[derive(Debug, Clone)]
struct Template {
    name: String,
    services: Vec<String>,
    files: Vec<TemplateFile>,
    service_configs: Vec<ServiceConfig>,
    packages: Vec<String>,
}

#[derive(Debug, Args)]
pub struct InitArgs {
    /// Project name
    #[arg()]
    pub name: Option<String>,
    
    /// Skip interactive prompts and use defaults
    #[arg(long)]
    pub yes: bool,
    
    /// Template to use
    #[arg(long)]
    pub template: Option<String>,
    
    /// Initialize with Docker support
    #[arg(long)]
    pub docker: bool,
}

impl InitArgs {
    pub async fn execute(&self) -> Result<()> {
        println!("🚀 Initializing new Devbox project...");

      if std::env::var("DEVBOX_DEBUG").is_ok() {
          self.list_available_templates();
      }

        let project_name = self.get_project_name().await?;
        let template = self.select_template().await?;
        let services = self.select_services(&template).await?;
        let with_docker = self.should_include_docker().await?;
        
        // Validate template services if we have a template config
        if let Some(template_config) = self.get_template_config(&template) {
            self.validate_template_services(&template_config, &services);
        }
        
        self.create_project_structure(&project_name, &template, &services, with_docker).await?;
        self.generate_devbox_yaml(&project_name, &template, &services, with_docker).await?;
        self.install_dependencies(&project_name, &services).await?;
        
        if with_docker {
            self.generate_docker_files(&project_name, &template).await?;
        }
        
        println!("✅ Successfully created project: {}", project_name);
        println!("📁 Project location: ./{}", project_name);
        println!("🚀 Get started with: cd {} && devbox start", project_name);
        
        Ok(())
    }

    async fn install_dependencies(&self, project_name: &str, services: &[String]) -> Result<()> {
        println!("📦 Installing dependencies...");
        
        for service in services {
            let service_dir = format!("{}/{}", project_name, service);
            
            if Path::new(&format!("{}/package.json", service_dir)).exists() {
                println!("   Installing dependencies for {}...", service);
                
                if Command::new("npm").arg("--version").output().is_ok() {
                    let status = Command::new("npm")
                        .arg("install")
                        .current_dir(&service_dir)
                        .status();
                    
                    match status {
                        Ok(exit_status) if exit_status.success() => {
                            println!("   ✅ Dependencies installed for {}", service);
                        }
                        Ok(exit_status) => {
                            println!("   ⚠️  Failed to install dependencies for {} (exit code: {})", service, exit_status);
                            println!("   💡 Run 'cd {} && npm install' manually", service_dir);
                        }
                        Err(e) => {
                            println!("   ⚠️  Could not run npm install for {}: {}", service, e);
                            println!("   💡 Make sure Node.js is installed and run 'cd {} && npm install'", service_dir);
                        }
                    }
                } else {
                    println!("   ⚠️  npm not available for {}", service);
                    println!("   💡 Install Node.js and run 'cd {} && npm install'", service_dir);
                }
            }
        }
        
        Ok(())
    }
    
    async fn get_project_name(&self) -> Result<String> {
        if let Some(name) = &self.name {
            return self.validate_project_name(name);
        }
        
        if self.yes {
            return Ok("my-devbox-project".to_string());
        }
        
        print!("📝 Project name: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        let name = input.trim();
        if name.is_empty() {
            return Ok("my-devbox-project".to_string());
        }
        
        self.validate_project_name(name)
    }

    fn validate_project_name(&self, name: &str) -> Result<String> {
        if name.is_empty() {
            return Ok("my-devbox-project".to_string());
        }
        
        if name.chars().any(|c| !c.is_ascii_alphanumeric() && c != '-' && c != '_') {
            return Err(crate::error::ToolError::ValidationError(
                "Project name can only contain letters, numbers, hyphens, and underscores".to_string()
            ));
        }
        
        if Path::new(name).exists() {
            return Err(crate::error::ToolError::ValidationError(
                format!("Directory '{}' already exists", name)
            ));
        }
        
        Ok(name.to_string())
    }
    
    async fn select_template(&self) -> Result<String> {
        if let Some(template) = &self.template {
            let normalized = match template.to_lowercase().as_str() {
                "nextjs" | "next.js" | "next" => "nextjs",
                "react" | "vite" => "react",
                "vue" => "vue", 
                "svelte" => "svelte",
                "node" | "express" => "node",
                "python" | "fastapi" => "python",
                "rust" | "axum" => "rust",
                "go" | "gin" => "go",
                "fullstack" | "microservices" | "custom" => template.as_str(),
                other => {
                    eprintln!("❌ Unknown template: {}. Using default (nextjs)", other);
                    "nextjs"
                }
            };
            return Ok(normalized.to_string());
        }

        if self.yes {
            return Ok("nextjs".to_string());
        }
        
        println!("\n📋 Select project template:");
        println!("1. Next.js (Modern React fullstack)");
        println!("2. React (Vite + TypeScript)");
        println!("3. Vue (Vite + TypeScript)");
        println!("4. Svelte (Vite + TypeScript)");
        println!("5. Node.js (Express API)");
        println!("6. Python (FastAPI)");
        println!("7. Rust (Axum Web API)");
        println!("8. Go (Gin Web API)");
        println!("9. Fullstack (Frontend + API + Database)");
        println!("10. Microservices (Multiple services)");
        println!("11. Custom (Choose individual services)");
        
        print!("Choose template [1-11]: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        match input.trim() {
            "1" => Ok("nextjs".to_string()),
            "2" => Ok("react".to_string()),
            "3" => Ok("vue".to_string()),
            "4" => Ok("svelte".to_string()),
            "5" => Ok("node".to_string()),
            "6" => Ok("python".to_string()),
            "7" => Ok("rust".to_string()),
            "8" => Ok("go".to_string()),
            "9" => Ok("fullstack".to_string()),
            "10" => Ok("microservices".to_string()),
            "11" => Ok("custom".to_string()),
            _ => Ok("custom".to_string()),
        }
    }

    async fn select_services(&self, template: &str) -> Result<Vec<String>> {
        if self.yes {
            return match template {
                "nextjs" => Ok(vec!["frontend".to_string()]),
                "react" => Ok(vec!["frontend".to_string()]),
                "vue" => Ok(vec!["frontend".to_string()]),
                "svelte" => Ok(vec!["frontend".to_string()]),
                "node" => Ok(vec!["api".to_string()]),
                "python" => Ok(vec!["api".to_string()]),
                "rust" => Ok(vec!["api".to_string()]),
                "go" => Ok(vec!["api".to_string()]),
                "api" => Ok(vec!["api".to_string()]),
                "fullstack" => Ok(vec!["frontend".to_string(), "api".to_string(), "database".to_string()]),
                _ => Ok(vec!["frontend".to_string(), "api".to_string()]),
            };
        }
        
        match template {
            "nextjs" => Ok(vec!["frontend".to_string()]),
            "react" => Ok(vec!["frontend".to_string()]),
            "vue" => Ok(vec!["frontend".to_string()]),
            "svelte" => Ok(vec!["frontend".to_string()]),
            "node" => Ok(vec!["api".to_string()]),
            "python" => Ok(vec!["api".to_string()]),
            "rust" => Ok(vec!["api".to_string()]),
            "go" => Ok(vec!["api".to_string()]),
            "web" => Ok(vec!["frontend".to_string(), "api".to_string()]),
            "api" => Ok(vec!["api".to_string()]),
            "fullstack" => Ok(vec!["frontend".to_string(), "api".to_string(), "database".to_string()]),
            "microservices" => Ok(vec!["frontend".to_string(), "api".to_string(), "auth".to_string(), "database".to_string()]),
            "custom" => self.select_custom_services().await,
            _ => Ok(vec!["frontend".to_string(), "api".to_string()]),
        }
    }

    async fn select_custom_services(&self) -> Result<Vec<String>> {
        println!("\n🛠️  Select services to include:");
        let services = ["frontend", "api", "database", "cache", "auth", "queue", "storage", "monitoring"];
        
        for (i, service) in services.iter().enumerate() {
            println!("{}. {}", i + 1, service);
        }
        
        print!("Select services (comma-separated, e.g., 1,2,3): ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        let selected: Vec<String> = input
            .trim()
            .split(',')
            .filter_map(|s| s.trim().parse::<usize>().ok())
            .filter_map(|i| services.get(i - 1))
            .map(|s| s.to_string())
            .collect();
        
        if selected.is_empty() {
            Ok(vec!["frontend".to_string(), "api".to_string()])
        } else {
            Ok(selected)
        }
    }
    
    async fn should_include_docker(&self) -> Result<bool> {
        if self.docker {
            return Ok(true);
        }
        
        if self.yes {
            return Ok(false);
        }
        
        print!("🐳 Include Docker support? [y/N]: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        Ok(input.trim().eq_ignore_ascii_case("y") || input.trim().eq_ignore_ascii_case("yes"))
    }
    
    async fn create_project_structure(&self, project_name: &str, template: &str, services: &[String], with_docker: bool) -> Result<()> {
        println!("📁 Creating project structure...");
        
        std::fs::create_dir_all(project_name)?;
        
        if let Some(template_config) = self.get_template_config(template) {
            println!("   Using {} template", template_config.name);
            println!("   Template services: {}", template_config.services.join(", "));
            self.create_template_files(project_name, &template_config).await?;
        } else {
            println!("   Using fallback structure for: {}", services.join(", "));
            self.create_fallback_structure(project_name, template, services).await?;
        }
        
        if with_docker {
            std::fs::create_dir_all(format!("{}/docker", project_name))?;
        }
        
        Ok(())
    }

    async fn create_template_files(&self, project_name: &str, template: &Template) -> Result<()> {
        for file in &template.files {
            let full_path = format!("{}/{}", project_name, file.path);
            
            if let Some(parent) = std::path::Path::new(&full_path).parent() {
                std::fs::create_dir_all(parent)?;
            }
            
            std::fs::write(&full_path, file.content)?;
        }
        Ok(())
    }

    async fn create_fallback_structure(&self, project_name: &str, _template: &str, services: &[String]) -> Result<()> {
        for service in services {
            let service_dir = format!("{}/{}", project_name, service);
            std::fs::create_dir_all(&service_dir)?;
            
            match service.as_str() {
                "frontend" => self.create_basic_frontend(&service_dir).await?,
                "api" => self.create_basic_api(&service_dir).await?,
                "database" => self.create_database(&service_dir).await?,
                _ => self.create_generic_service(&service_dir, service).await?,
            }
        }
        Ok(())
    }

    async fn generate_devbox_yaml(&self, project_name: &str, template: &str, services: &[String], with_docker: bool) -> Result<()> {
        println!("📄 Generating devbox.yaml...");
        
        let mut yaml_content = format!(
            "name: \"{}\"\ndescription: \"{} project\"\n\n",
            project_name, template
        );
        
        yaml_content.push_str("packages:\n");
        
        if let Some(template_config) = self.get_template_config(template) {
            println!("   Configuring packages for {} template", template_config.name);
            for package in &template_config.packages {
                yaml_content.push_str(&format!("  {}\n", package));
            }
        } else {
            match template {
                "python" => yaml_content.push_str("  python@latest\n  pip@latest\n"),
                "rust" => yaml_content.push_str("  rustup@latest\n"),
                "go" => yaml_content.push_str("  go@latest\n"),
                _ => yaml_content.push_str("  nodejs@latest\n  npm@latest\n"),
            }
        }
        
        yaml_content.push_str("\ncommands:\n  start:\n    dev: \"echo 'Starting development environment'\"\n    build: \"echo 'Building project'\"\n    test: \"echo 'Running tests'\"\n\n");
        
        yaml_content.push_str("services:\n");
        
        if let Some(template_config) = self.get_template_config(template) {
            println!("   Configuring services for {} template", template_config.name);
            for service_config in &template_config.service_configs {
                yaml_content.push_str(&self.service_config_to_yaml(service_config));
            }
        } else {
            for service in services {
                let service_config = self.get_service_config(service, template);
                yaml_content.push_str(service_config);
                yaml_content.push('\n');
            }
        }
        
        if with_docker {
            yaml_content.push_str("\nenvironment:\n  DOCKER_ENABLED: \"true\"\n");
        }
        
        yaml_content.push_str("\nhooks:\n  pre_start: \"echo 'Setting up development environment'\"\n  post_start: \"echo 'All services are ready!'\"\n");
        
        std::fs::write(format!("{}/devbox.yaml", project_name), yaml_content)?;
        Ok(())
    }

    fn service_config_to_yaml(&self, config: &ServiceConfig) -> String {
        format!(
            "  - name: \"{}\"\n    service_type: \"{}\"\n    command: \"{}\"\n    working_dir: \"{}\"\n    health_check:\n      type_entry: \"{}\"\n      port: {}\n      http_target: \"{}\"\n    dependencies: [{}]\n",
            config.name,
            config.service_type,
            config.command,
            config.working_dir,
            config.health_check.type_entry,
            config.health_check.port,
            config.health_check.http_target,
            config.dependencies.join(", ")
        )
    }

    async fn generate_docker_files(&self, project_name: &str, template: &str) -> Result<()> {
        println!("🐳 Generating Docker files...");
        
        let dockerfile_frontend = match template {
            "nextjs" => DOCKERFILE_NEXTJS,
            _ => DOCKERFILE_FRONTEND,
        };
        
        std::fs::write(
            format!("{}/docker/Dockerfile.frontend", project_name),
            dockerfile_frontend
        )?;
        
        std::fs::write(
            format!("{}/docker/Dockerfile.api", project_name),
            DOCKERFILE_API
        )?;
        
        std::fs::write(
            format!("{}/docker-compose.yml", project_name),
            DOCKER_COMPOSE
        )?;
        
        std::fs::write(
            format!("{}/.dockerignore", project_name),
            DOCKER_IGNORE
        )?;
        
        Ok(())
    }

  pub fn list_available_templates(&self) {
      println!("🎯 Available Devbox Templates:");
      println!("{:-<50}", "");
      
      let templates = [
          ("nextjs", "Next.js Fullstack"),
          ("react", "React Frontend"),
          ("vue", "Vue Frontend"), 
          ("svelte", "Svelte Frontend"),
          ("node", "Node.js API"),
          ("python", "Python FastAPI"),
          ("rust", "Rust Axum API"),
          ("go", "Go Gin API"),
          ("fullstack", "Fullstack App"),
      ];
      
      for (template_key, template_description) in templates {
          if let Some(template) = self.get_template_config(template_key) {
              println!("📦 {}", template_description);
              println!("   Key: {}", template_key);
              println!("   Services: {}", template.services.join(", "));
              println!("   Packages: {}", template.packages.join(", "));
              println!();
          }
      }
  }

    fn validate_template_services(&self, template: &Template, selected_services: &[String]) -> bool {
        for service in selected_services {
            if !template.services.contains(service) {
                eprintln!("Warning: Service '{}' not typically part of {} template", service, template.name);
                return false;
            }
        }
        true
    }

    // Template configuration
    fn get_template_config(&self, template_name: &str) -> Option<Template> {
        match template_name {
            "nextjs" => Some(self.nextjs_template()),
            "react" => Some(self.react_template()),
            "vue" => Some(self.vue_template()),
            "svelte" => Some(self.svelte_template()),
            "node" => Some(self.node_template()),
            "python" => Some(self.python_template()),
            "rust" => Some(self.rust_template()),
            "go" => Some(self.go_template()),
            "fullstack" => Some(self.fullstack_template()),
            _ => None,
        }
    }

    fn nextjs_template(&self) -> Template {
        Template {
            name: "nextjs".to_string(),
            services: vec!["frontend".to_string()],
            packages: vec!["nodejs@latest".to_string(), "npm@latest".to_string()],
            files: vec![
                TemplateFile {
                    path: "frontend/package.json".to_string(),
                    content: NEXTJS_PACKAGE_JSON,
                },
                TemplateFile {
                    path: "frontend/next.config.js".to_string(),
                    content: NEXTJS_CONFIG,
                },
                TemplateFile {
                    path: "frontend/tsconfig.json".to_string(),
                    content: NEXTJS_TS_CONFIG,
                },
                TemplateFile {
                    path: "frontend/app/globals.css".to_string(),  // ADD THIS
                    content: NEXTJS_GLOBALS_CSS,
                },
                TemplateFile {
                    path: "frontend/app/layout.tsx".to_string(),
                    content: NEXTJS_LAYOUT,
                },
                TemplateFile {
                    path: "frontend/app/page.tsx".to_string(),
                    content: NEXTJS_HOME_PAGE,
                },
                TemplateFile {
                    path: "frontend/app/api/hello/route.ts".to_string(),
                    content: NEXTJS_API_ROUTE,
                },
                TemplateFile {
                    path: "frontend/tailwind.config.js".to_string(),
                    content: NEXTJS_TAILWIND_CONFIG,
                },
                TemplateFile {
                    path: "frontend/postcss.config.js".to_string(),
                    content: NEXTJS_POSTCSS_CONFIG,
                },
            ],
            service_configs: vec![ServiceConfig {
                name: "frontend".to_string(),
                service_type: "web".to_string(),
                command: "cd frontend && npm run dev".to_string(),
                working_dir: "./frontend".to_string(),
                health_check: HealthCheck {
                    type_entry: "http".to_string(),
                    port: 3000,
                    http_target: "http://localhost:3000".to_string(),
                },
                dependencies: vec![],
            }],
        }
    }

    fn vue_template(&self) -> Template {
        Template {
            name: "vue".to_string(),
            services: vec!["frontend".to_string()],
            packages: vec!["nodejs@latest".to_string(), "npm@latest".to_string()],
            files: vec![
                TemplateFile {
                    path: "frontend/package.json".to_string(),
                    content: VUE_PACKAGE_JSON,
                },
                TemplateFile {
                    path: "frontend/vite.config.ts".to_string(),
                    content: VUE_VITE_CONFIG,
                },
                TemplateFile {
                    path: "frontend/tsconfig.json".to_string(),
                    content: VUE_TS_CONFIG,
                },
                TemplateFile {
                    path: "frontend/tsconfig.node.json".to_string(), // ADD THIS
                    content: VUE_TS_CONFIG_NODE,
                },
                TemplateFile {
                    path: "frontend/index.html".to_string(),
                    content: VUE_HTML,
                },
                TemplateFile {
                    path: "frontend/src/main.ts".to_string(),
                    content: VUE_MAIN,
                },
                TemplateFile {
                    path: "frontend/src/App.vue".to_string(),
                    content: VUE_APP,
                },
                TemplateFile {
                    path: "frontend/src/components/HelloWorld.vue".to_string(),
                    content: VUE_HELLO_WORLD,
                },
                TemplateFile {
                    path: "frontend/src/style.css".to_string(),
                    content: VUE_STYLE_CSS,
                },
                TemplateFile {
                    path: "frontend/src/vite-env.d.ts".to_string(),
                    content: VUE_VITE_ENV,
                },
            ],
            service_configs: vec![ServiceConfig {
                name: "frontend".to_string(),
                service_type: "web".to_string(),
                command: "cd frontend && npm run dev".to_string(),
                working_dir: "./frontend".to_string(),
                health_check: HealthCheck {
                    type_entry: "http".to_string(),
                    port: 5173,
                    http_target: "http://localhost:5173".to_string(),
                },
                dependencies: vec![],
            }],
        }
    }



    fn react_template(&self) -> Template {
        Template {
            name: "react".to_string(),
            services: vec!["frontend".to_string()],
            packages: vec!["nodejs@latest".to_string(), "npm@latest".to_string()],
            files: vec![
                TemplateFile {
                    path: "frontend/package.json".to_string(),
                    content: REACT_PACKAGE_JSON,
                },
                TemplateFile {
                    path: "frontend/vite.config.ts".to_string(),
                    content: REACT_VITE_CONFIG,
                },
                TemplateFile {
                    path: "frontend/tsconfig.json".to_string(),
                    content: REACT_TS_CONFIG,
                },
                TemplateFile {
                    path: "frontend/src/main.tsx".to_string(),
                    content: REACT_MAIN,
                },
                TemplateFile {
                    path: "frontend/src/App.tsx".to_string(),
                    content: REACT_APP,
                },
                TemplateFile {
                    path: "frontend/src/vite-env.d.ts".to_string(),
                    content: REACT_VITE_ENV,
                },
                TemplateFile {
                    path: "frontend/src/index.css".to_string(),
                    content: REACT_INDEX_CSS,
                },
                TemplateFile {
                    path: "frontend/src/App.css".to_string(),
                    content: REACT_APP_CSS,
                },
                TemplateFile {
                    path: "frontend/index.html".to_string(),
                    content: REACT_HTML,
                },
            ],
            service_configs: vec![ServiceConfig {
                name: "frontend".to_string(),
                service_type: "web".to_string(),
                command: "cd frontend && npm run dev".to_string(),
                working_dir: "./frontend".to_string(),
                health_check: HealthCheck {
                    type_entry: "http".to_string(),
                    port: 5173,
                    http_target: "http://localhost:5173".to_string(),
                },
                dependencies: vec![],
            }],
        }
    }

    fn svelte_template(&self) -> Template {
        Template {
            name: "svelte".to_string(),
            services: vec!["frontend".to_string()],
            packages: vec!["nodejs@latest".to_string(), "npm@latest".to_string()],
            files: vec![
                TemplateFile {
                    path: "frontend/package.json".to_string(),
                    content: SVELTE_PACKAGE_JSON,
                },
                TemplateFile {
                    path: "frontend/svelte.config.js".to_string(),
                    content: SVELTE_CONFIG,
                },
                TemplateFile {
                    path: "frontend/vite.config.ts".to_string(),
                    content: SVELTE_VITE_CONFIG,
                },
                TemplateFile {
                    path: "frontend/tsconfig.json".to_string(),
                    content: SVELTE_TS_CONFIG,
                },
                TemplateFile {
                    path: "frontend/src/app.html".to_string(),
                    content: SVELTE_APP_HTML,
                },
                TemplateFile {
                    path: "frontend/src/main.ts".to_string(),
                    content: SVELTE_MAIN,
                },
                TemplateFile {
                    path: "frontend/src/app.css".to_string(),
                    content: SVELTE_APP_CSS,
                },
            ],
            service_configs: vec![ServiceConfig {
                name: "frontend".to_string(),
                service_type: "web".to_string(),
                command: "cd frontend && npm run dev".to_string(),
                working_dir: "./frontend".to_string(),
                health_check: HealthCheck {
                    type_entry: "http".to_string(),
                    port: 5173,
                    http_target: "http://localhost:5173".to_string(),
                },
                dependencies: vec![],
            }],
        }
    }

    fn node_template(&self) -> Template {
        Template {
            name: "node".to_string(),
            services: vec!["api".to_string()],
            packages: vec!["nodejs@latest".to_string(), "npm@latest".to_string()],
            files: vec![
                TemplateFile {
                    path: "api/package.json".to_string(),
                    content: NODE_API_PACKAGE_JSON,
                },
                TemplateFile {
                    path: "api/server.js".to_string(),
                    content: NODE_API_SERVER,
                },
            ],
            service_configs: vec![ServiceConfig {
                name: "api".to_string(),
                service_type: "api".to_string(),
                command: "cd api && npm run dev".to_string(),
                working_dir: "./api".to_string(),
                health_check: HealthCheck {
                    type_entry: "http".to_string(),
                    port: 3001,
                    http_target: "http://localhost:3001/health".to_string(),
                },
                dependencies: vec![],
            }],
        }
    }

    fn python_template(&self) -> Template {
        Template {
            name: "python".to_string(),
            services: vec!["api".to_string()],
            packages: vec!["python@latest".to_string(), "pip@latest".to_string()],
            files: vec![
                TemplateFile {
                    path: "api/requirements.txt".to_string(),
                    content: PYTHON_REQUIREMENTS,
                },
                TemplateFile {
                    path: "api/main.py".to_string(),
                    content: PYTHON_MAIN,
                },
            ],
            service_configs: vec![ServiceConfig {
                name: "api".to_string(),
                service_type: "api".to_string(),
                command: "cd api && python main.py".to_string(),
                working_dir: "./api".to_string(),
                health_check: HealthCheck {
                    type_entry: "http".to_string(),
                    port: 8000,
                    http_target: "http://localhost:8000/health".to_string(),
                },
                dependencies: vec![],
            }],
        }
    }

    fn rust_template(&self) -> Template {
        Template {
            name: "rust".to_string(),
            services: vec!["api".to_string()],
            packages: vec!["rustup@latest".to_string()],
            files: vec![
                TemplateFile {
                    path: "api/Cargo.toml".to_string(),
                    content: RUST_CARGO_TOML,
                },
                TemplateFile {
                    path: "api/src/main.rs".to_string(),
                    content: RUST_MAIN,
                },
            ],
            service_configs: vec![ServiceConfig {
                name: "api".to_string(),
                service_type: "api".to_string(),
                command: "cd api && cargo run".to_string(),
                working_dir: "./api".to_string(),
                health_check: HealthCheck {
                    type_entry: "http".to_string(),
                    port: 8080,
                    http_target: "http://localhost:8080/health".to_string(),
                },
                dependencies: vec![],
            }],
        }
    }

    fn go_template(&self) -> Template {
        Template {
            name: "go".to_string(),
            services: vec!["api".to_string()],
            packages: vec!["go@latest".to_string()],
            files: vec![
                TemplateFile {
                    path: "api/go.mod".to_string(),
                    content: GO_MOD,
                },
                TemplateFile {
                    path: "api/main.go".to_string(),
                    content: GO_MAIN,
                },
            ],
            service_configs: vec![ServiceConfig {
                name: "api".to_string(),
                service_type: "api".to_string(),
                command: "cd api && go run main.go".to_string(),
                working_dir: "./api".to_string(),
                health_check: HealthCheck {
                    type_entry: "http".to_string(),
                    port: 9090,
                    http_target: "http://localhost:9090/health".to_string(),
                },
                dependencies: vec![],
            }],
        }
    }

    fn fullstack_template(&self) -> Template {
        Template {
            name: "fullstack".to_string(),
            services: vec!["frontend".to_string(), "api".to_string(), "database".to_string()],
            packages: vec!["nodejs@latest".to_string(), "npm@latest".to_string()],
            files: vec![],
            service_configs: vec![
                ServiceConfig {
                    name: "frontend".to_string(),
                    service_type: "web".to_string(),
                    command: "cd frontend && npm run dev".to_string(),
                    working_dir: "./frontend".to_string(),
                    health_check: HealthCheck {
                        type_entry: "http".to_string(),
                        port: 5173,
                        http_target: "http://localhost:5173".to_string(),
                    },
                    dependencies: vec![],
                },
                ServiceConfig {
                    name: "api".to_string(),
                    service_type: "api".to_string(),
                    command: "cd api && npm run dev".to_string(),
                    working_dir: "./api".to_string(),
                    health_check: HealthCheck {
                        type_entry: "http".to_string(),
                        port: 3001,
                        http_target: "http://localhost:3001/health".to_string(),
                    },
                    dependencies: vec![],
                },
                ServiceConfig {
                    name: "database".to_string(),
                    service_type: "database".to_string(),
                    command: "docker run -p 5432:5432 -e POSTGRES_PASSWORD=devbox postgres:15".to_string(),
                    working_dir: "./database".to_string(),
                    health_check: HealthCheck {
                        type_entry: "port".to_string(),
                        port: 5432,
                        http_target: "".to_string(),
                    },
                    dependencies: vec![],
                },
            ],
        }
    }

    // Fallback file creation methods
    async fn create_basic_frontend(&self, service_dir: &str) -> Result<()> {
        std::fs::write(format!("{}/package.json", service_dir), BASIC_FRONTEND_PACKAGE_JSON)?;
        std::fs::write(format!("{}/index.html", service_dir), BASIC_FRONTEND_HTML)?;
        Ok(())
    }

    async fn create_basic_api(&self, service_dir: &str) -> Result<()> {
        std::fs::write(format!("{}/package.json", service_dir), BASIC_API_PACKAGE_JSON)?;
        std::fs::write(format!("{}/server.js", service_dir), BASIC_API_SERVER_JS)?;
        Ok(())
    }

    async fn create_database(&self, service_dir: &str) -> Result<()> {
        std::fs::write(format!("{}/init.sql", service_dir), DATABASE_INIT_SQL)?;
        Ok(())
    }

    async fn create_generic_service(&self, service_dir: &str, service_name: &str) -> Result<()> {
        std::fs::write(
            format!("{}/README.md", service_dir),
            format!("# {}\n\nService configuration.", service_name)
        )?;
        Ok(())
    }

    fn get_service_config(&self, service: &str, template: &str) -> &'static str {
        match (service, template) {
            ("frontend", "nextjs") => NEXTJS_SERVICE_CONFIG,
            ("frontend", "react") => REACT_SERVICE_CONFIG,
            ("frontend", "vue") => VUE_SERVICE_CONFIG,
            ("frontend", "svelte") => SVELTE_SERVICE_CONFIG,
            ("frontend", _) => FRONTEND_SERVICE_CONFIG,
            ("api", "node") => NODE_API_SERVICE_CONFIG,
            ("api", "python") => PYTHON_API_SERVICE_CONFIG,
            ("api", "rust") => RUST_API_SERVICE_CONFIG,
            ("api", "go") => GO_API_SERVICE_CONFIG,
            ("api", _) => API_SERVICE_CONFIG,
            ("database", _) => DATABASE_SERVICE_CONFIG,
            ("cache", _) => CACHE_SERVICE_CONFIG,
            ("auth", _) => AUTH_SERVICE_CONFIG,
            _ => GENERIC_SERVICE_CONFIG,
        }
    }
}

// ========== TEMPLATE CONSTANTS ==========


const NEXTJS_CONFIG: &str = r#"/** @type {import('next').NextConfig} */
const nextConfig = {
  experimental: {
    appDir: true,
  },
}

module.exports = nextConfig"#;

const NEXTJS_TS_CONFIG: &str = r#"{
  "compilerOptions": {
    "target": "es5",
    "lib": ["dom", "dom.iterable", "es6"],
    "allowJs": true,
    "skipLibCheck": true,
    "strict": true,
    "noEmit": true,
    "esModuleInterop": true,
    "module": "esnext",
    "moduleResolution": "bundler",
    "resolveJsonModule": true,
    "isolatedModules": true,
    "jsx": "preserve",
    "incremental": true,
    "plugins": [
      {
        "name": "next"
      }
    ],
    "baseUrl": ".",
    "paths": {
      "@/*": ["./*"]
    }
  },
  "include": ["next-env.d.ts", "**/*.ts", "**/*.tsx", ".next/types/**/*.ts"],
  "exclude": ["node_modules"]
}"#;

const NEXTJS_GLOBALS_CSS: &str = r#"@tailwind base;
@tailwind components;
@tailwind utilities;

:root {
  --foreground-rgb: 0, 0, 0;
  --background-start-rgb: 214, 219, 220;
  --background-end-rgb: 255, 255, 255;
}

@media (prefers-color-scheme: dark) {
  :root {
    --foreground-rgb: 255, 255, 255;
    --background-start-rgb: 0, 0, 0;
    --background-end-rgb: 0, 0, 0;
  }
}

body {
  color: rgb(var(--foreground-rgb));
  background: linear-gradient(
      to bottom,
      transparent,
      rgb(var(--background-end-rgb))
    )
    rgb(var(--background-start-rgb));
}

@layer utilities {
  .text-balance {
    text-wrap: balance;
  }
}"#;

const NEXTJS_LAYOUT: &str = r#"import './globals.css'

export const metadata = {
  title: 'Devbox Next.js App',
  description: 'Generated by Devbox CLI',
}

export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  )
}"#;

const NEXTJS_PACKAGE_JSON: &str = r#"{
  "name": "frontend",
  "version": "1.0.0",
  "scripts": {
    "dev": "next dev",
    "build": "next build",
    "start": "next start",
    "lint": "next lint"
  },
  "dependencies": {
    "next": "14.0.0",
    "react": "^18.0.0",
    "react-dom": "^18.0.0"
  },
  "devDependencies": {
    "typescript": "^5.0.0",
    "@types/node": "^20.0.0",
    "@types/react": "^18.0.0",
    "@types/react-dom": "^18.0.0",
    "eslint": "^8.0.0",
    "eslint-config-next": "14.0.0",
    "tailwindcss": "^3.0.0",
    "@tailwindcss/typography": "^0.5.0",
    "autoprefixer": "^10.0.0",
    "postcss": "^8.0.0"
  }
}"#;

const NEXTJS_TAILWIND_CONFIG: &str = r#"/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    './pages/**/*.{js,ts,jsx,tsx,mdx}',
    './components/**/*.{js,ts,jsx,tsx,mdx}',
    './app/**/*.{js,ts,jsx,tsx,mdx}',
  ],
  theme: {
    extend: {
      backgroundImage: {
        'gradient-radial': 'radial-gradient(var(--tw-gradient-stops))',
        'gradient-conic':
          'conic-gradient(from 180deg at 50% 50%, var(--tw-gradient-stops))',
      },
    },
  },
  plugins: [],
}"#;

const NEXTJS_POSTCSS_CONFIG: &str = r#"module.exports = {
  plugins: {
    tailwindcss: {},
    autoprefixer: {},
  },
}"#;

const NEXTJS_HOME_PAGE: &str = r#"import React from 'react'
export default function Home() {
  return (
    <main className="min-h-screen bg-gradient-to-br from-blue-50 to-indigo-100 flex items-center justify-center">
      <div className="text-center">
        <h1 className="text-6xl font-bold text-gray-800 mb-4">
          Welcome to Devbox
        </h1>
        <p className="text-xl text-gray-600 mb-8">
          Your Next.js app is running successfully!
        </p>
        <div className="bg-white rounded-lg shadow-lg p-6 max-w-md mx-auto">
          <p className="text-gray-700">
            Get started by editing <code className="bg-gray-100 px-2 py-1 rounded">app/page.tsx</code>
          </p>
        </div>
      </div>
    </main>
  )
}"#;

const NEXTJS_API_ROUTE: &str = r#"import { NextResponse } from 'next/server'

export async function GET() {
  return NextResponse.json({ message: 'Hello from Next.js API!' })
}"#;

// React Templates
const REACT_PACKAGE_JSON: &str = r#"{
  "name": "frontend",
  "version": "1.0.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "react": "^18.0.0",
    "react-dom": "^18.0.0"
  },
  "devDependencies": {
    "@types/react": "^18.0.0",
    "@types/react-dom": "^18.0.0",
    "@vitejs/plugin-react": "^4.0.0",
    "typescript": "^5.0.0",
    "vite": "^5.0.0"
  }
}"#;

const REACT_VITE_CONFIG: &str = r#"import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    host: true
  }
})"#;

const REACT_TS_CONFIG: &str = r#"{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true
  },
  "include": ["src"],
  "references": [{ "path": "./tsconfig.node.json" }]
}"#;

const REACT_MAIN: &str = r#"import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App.tsx'
import './index.css'

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
)"#;

const REACT_APP: &str = r#"import { useState } from 'react'
import reactLogo from './assets/react.svg'
import './App.css'

function App() {
  const [count, setCount] = useState(0)

  return (
    <div className="App">
      <div>
        <a href="https://vitejs.dev" target="_blank">
          <img src="/vite.svg" className="logo" alt="Vite logo" />
        </a>
        <a href="https://reactjs.org" target="_blank">
          <img src={reactLogo} className="logo react" alt="React logo" />
        </a>
      </div>
      <h1>Vite + React</h1>
      <div className="card">
        <button onClick={() => setCount((count) => count + 1)}>
          count is {count}
        </button>
        <p>
          Edit <code>src/App.tsx</code> and save to test HMR
        </p>
      </div>
      <p className="read-the-docs">
        Click on the Vite and React logos to learn more
      </p>
    </div>
  )
}

export default App"#;

const REACT_VITE_ENV: &str = r#"/// <reference types="vite/client" />"#;

const REACT_INDEX_CSS: &str = r#":root {
  font-family: Inter, system-ui, Avenir, Helvetica, Arial, sans-serif;
  line-height: 1.5;
  font-weight: 400;

  color-scheme: light dark;
  color: rgba(255, 255, 255, 0.87);
  background-color: #242424;

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

a {
  font-weight: 500;
  color: #646cff;
  text-decoration: inherit;
}
a:hover {
  color: #535bf2;
}

body {
  margin: 0;
  display: flex;
  place-items: center;
  min-width: 320px;
  min-height: 100vh;
}

h1 {
  font-size: 3.2em;
  line-height: 1.1;
}

button {
  border-radius: 8px;
  border: 1px solid transparent;
  padding: 0.6em 1.2em;
  font-size: 1em;
  font-weight: 500;
  font-family: inherit;
  background-color: #1a1a1a;
  cursor: pointer;
  transition: border-color 0.25s;
}
button:hover {
  border-color: #646cff;
}
button:focus,
button:focus-visible {
  outline: 4px auto -webkit-focus-ring-color;
}

@media (prefers-color-scheme: light) {
  :root {
    color: #213547;
    background-color: #ffffff;
  }
  a:hover {
    color: #747bff;
  }
  button {
    background-color: #f9f9f9;
  }
}"#;

const REACT_APP_CSS: &str = r#".App {
  text-align: center;
}

.App-logo {
  height: 40vmin;
  pointer-events: none;
}

@media (prefers-reduced-motion: no-preference) {
  .App-logo {
    animation: App-logo-spin infinite 20s linear;
  }
}

.App-header {
  background-color: #282c34;
  padding: 20px;
  color: white;
}

.App-link {
  color: #61dafb;
}

@keyframes App-logo-spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

.card {
  padding: 2em;
}

.read-the-docs {
  color: #888;
}"#;

const REACT_HTML: &str = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <link rel="icon" type="image/svg+xml" href="/vite.svg" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Devbox React App</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>"#;

// Vue Templates
const VUE_PACKAGE_JSON: &str = r#"{
  "name": "frontend",
  "version": "1.0.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vue-tsc && vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "vue": "^3.3.0"
  },
  "devDependencies": {
    "@vitejs/plugin-vue": "^4.0.0",
    "@tsconfig/node18": "^18.0.0",
    "typescript": "^5.0.0",
    "vue-tsc": "^1.0.0",
    "vite": "^5.0.0"
  }
}"#;

const VUE_TS_CONFIG_NODE: &str = r#"{
  "extends": "@tsconfig/node18/tsconfig.json",
  "include": [
    "vite.config.*",
    "vitest.config.*",
    "cypress.config.*",
    "nightwatch.conf.*",
    "playwright.config.*"
  ],
  "compilerOptions": {
    "composite": true,
    "module": "ESNext",
    "types": ["node"]
  }
}"#;


const VUE_VITE_CONFIG: &str = r#"import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [vue()],
  server: {
    port: 5173,
    host: true
  },
  resolve: {
    alias: {
      '@': '/src'
    }
  }
})"#;


const VUE_TS_CONFIG: &str = r#"{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "preserve",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true
  },
  "include": ["src/**/*.ts", "src/**/*.tsx", "src/**/*.vue"],
  "references": [{ "path": "./tsconfig.node.json" }]
}"#;

const VUE_MAIN: &str = r#"import { createApp } from 'vue'
import './style.css'
import App from './App.vue'

createApp(App).mount('#app')"#;


const VUE_APP: &str = r#"<template>
  <div id="app">
    <img alt="Vue logo" src="./assets/logo.png" width="125" height="125" />
    <HelloWorld msg="Welcome to Your Vue.js + TypeScript App" />
  </div>
</template>

<script setup lang="ts">
import HelloWorld from './components/HelloWorld.vue'
</script>

<style>
#app {
  font-family: Avenir, Helvetica, Arial, sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  text-align: center;
  color: #2c3e50;
  margin-top: 60px;
}
</style>"#;

const VUE_HELLO_WORLD: &str = r#"<template>
  <div class="hello">
    <h1>{{ msg }}</h1>
    <p>
      For a guide and recipes on how to configure / customize this project,<br>
      check out the
      <a href="https://cli.vuejs.org" target="_blank" rel="noopener">vue-cli documentation</a>.
    </p>
    <h3>Installed CLI Plugins</h3>
    <ul>
      <li><a href="https://github.com/vuejs/vue-cli/tree/dev/packages/%40vue/cli-plugin-typescript" target="_blank" rel="noopener">typescript</a></li>
    </ul>
    <h3>Essential Links</h3>
    <ul>
      <li><a href="https://vuejs.org" target="_blank" rel="noopener">Core Docs</a></li>
      <li><a href="https://forum.vuejs.org" target="_blank" rel="noopener">Forum</a></li>
      <li><a href="https://chat.vuejs.org" target="_blank" rel="noopener">Community Chat</a></li>
      <li><a href="https://twitter.com/vuejs" target="_blank" rel="noopener">Twitter</a></li>
      <li><a href="https://news.vuejs.org" target="_blank" rel="noopener">News</a></li>
    </ul>
    <h3>Ecosystem</h3>
    <ul>
      <li><a href="https://router.vuejs.org" target="_blank" rel="noopener">vue-router</a></li>
      <li><a href="https://vuex.vuejs.org" target="_blank" rel="noopener">vuex</a></li>
      <li><a href="https://github.com/vuejs/vue-devtools#vue-devtools" target="_blank" rel="noopener">vue-devtools</a></li>
      <li><a href="https://vue-loader.vuejs.org" target="_blank" rel="noopener">vue-loader</a></li>
      <li><a href="https://github.com/vuejs/awesome-vue" target="_blank" rel="noopener">awesome-vue</a></li>
    </ul>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  msg: string
}>()
</script>

<style scoped>
h3 {
  margin: 40px 0 0;
}
ul {
  list-style-type: none;
  padding: 0;
}
li {
  display: inline-block;
  margin: 0 10px;
}
a {
  color: #42b983;
}
</style>"#;

const VUE_STYLE_CSS: &str = r#":root {
  font-family: Inter, system-ui, Avenir, Helvetica, Arial, sans-serif;
  line-height: 1.5;
  font-weight: 400;

  color-scheme: light dark;
  color: rgba(255, 255, 255, 0.87);
  background-color: #242424;

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

a {
  font-weight: 500;
  color: #646cff;
  text-decoration: inherit;
}
a:hover {
  color: #535bf2;
}

body {
  margin: 0;
  display: flex;
  place-items: center;
  min-width: 320px;
  min-height: 100vh;
}

h1 {
  font-size: 3.2em;
  line-height: 1.1;
}

button {
  border-radius: 8px;
  border: 1px solid transparent;
  padding: 0.6em 1.2em;
  font-size: 1em;
  font-weight: 500;
  font-family: inherit;
  background-color: #1a1a1a;
  cursor: pointer;
  transition: border-color 0.25s;
}
button:hover {
  border-color: #646cff;
}
button:focus,
button:focus-visible {
  outline: 4px auto -webkit-focus-ring-color;
}

.card {
  padding: 2em;
}

#app {
  max-width: 1280px;
  margin: 0 auto;
  padding: 2rem;
  text-align: center;
}

@media (prefers-color-scheme: light) {
  :root {
    color: #213547;
    background-color: #ffffff;
  }
  a:hover {
    color: #747bff;
  }
  button {
    background-color: #f9f9f9;
  }
}"#;

const VUE_VITE_ENV: &str = r#"/// <reference types="vite/client" />

declare module '*.vue' {
  import type { DefineComponent } from 'vue'
  const component: DefineComponent<{}, {}, any>
  export default component
}"#;

const VUE_HTML: &str = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <link rel="icon" type="image/svg+xml" href="/vite.svg" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Devbox Vue App</title>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>"#;

// Svelte Templates
const SVELTE_PACKAGE_JSON: &str = r#"{
  "name": "frontend",
  "version": "1.0.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview"
  },
  "devDependencies": {
    "@sveltejs/vite-plugin-svelte": "^2.0.0",
    "@tsconfig/svelte": "^5.0.0",
    "svelte": "^4.0.0",
    "svelte-check": "^3.0.0",
    "tslib": "^2.4.1",
    "typescript": "^5.0.0",
    "vite": "^5.0.0"
  }
}"#;

const SVELTE_CONFIG: &str = r#"import { vitePreprocess } from '@sveltejs/vite-plugin-svelte'

export default {
  // Consult https://svelte.dev/docs#compile-time-svelte-preprocess
  // for more information about preprocessors
  preprocess: vitePreprocess(),
}"#;

const SVELTE_VITE_CONFIG: &str = r#"import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'

export default defineConfig({
  plugins: [svelte()],
  server: {
    port: 5173,
    host: true
  }
})"#;

const SVELTE_TS_CONFIG: &str = r#"{
  "extends": "@tsconfig/svelte/tsconfig.json",
  "compilerOptions": {
    "target": "ESNext",
    "useDefineForClassFields": true,
    "module": "ESNext",
    "resolveJsonModule": true,
    /**
     * Typecheck JS in `.svelte` and `.js` files by default.
     * Disable checkJs if you'd like to use dynamic types in JS.
     * Note that setting allowJs false does not prevent the use
     * of JS in `.svelte` files.
     */
    "allowJs": true,
    "checkJs": true,
    "isolatedModules": true
  },
  "include": ["src/**/*.d.ts", "src/**/*.ts", "src/**/*.js", "src/**/*.svelte"],
  "references": [{ "path": "./tsconfig.node.json" }]
}"#;

const SVELTE_APP_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <link rel="icon" href="%sveltekit.assets%/favicon.png" />
    <meta name="viewport" content="width=device-width" />
    <title>Devbox Svelte App</title>
  </head>
  <body data-sveltekit-preload-data="hover">
    <div style="display: contents">%sveltekit.body%</div>
  </body>
</html>"#;

const SVELTE_MAIN: &str = r#"import { mount } from 'svelte'
import App from './App.svelte'
import './app.css'

const app = mount(App, {
  target: document.getElementById('app')!,
})

export default app"#;

const SVELTE_APP_CSS: &str = r#":root {
  font-family: Inter, system-ui, Avenir, Helvetica, Arial, sans-serif;
  line-height: 1.5;
  font-weight: 400;

  color-scheme: light dark;
  color: rgba(255, 255, 255, 0.87);
  background-color: #242424;

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

body {
  margin: 0;
  display: flex;
  place-items: center;
  min-width: 320px;
  min-height: 100vh;
}

#app {
  max-width: 1280px;
  margin: 0 auto;
  padding: 2rem;
  text-align: center;
}"#;

// Node.js API Templates
const NODE_API_PACKAGE_JSON: &str = r#"{
  "name": "api",
  "version": "1.0.0",
  "scripts": {
    "dev": "node server.js",
    "start": "node server.js"
  },
  "dependencies": {
    "express": "^4.18.0"
  }
}"#;

const NODE_API_SERVER: &str = r#"const express = require('express');
const app = express();
const port = process.env.PORT || 3001;

app.use(express.json());

app.get('/', (req, res) => {
  res.json({ message: 'Hello from Devbox Node.js API!' });
});

app.get('/health', (req, res) => {
  res.json({ status: 'OK', timestamp: new Date().toISOString() });
});

app.listen(port, () => {
  console.log(`API server running on port ${port}`);
});"#;

// Python API Templates
const PYTHON_REQUIREMENTS: &str = r#"fastapi==0.104.0
uvicorn==0.24.0"#;

const PYTHON_MAIN: &str = r#"from fastapi import FastAPI
from datetime import datetime

app = FastAPI()

@app.get("/")
async def root():
    return {"message": "Hello from Devbox Python API!"}

@app.get("/health")
async def health():
    return {"status": "OK", "timestamp": datetime.now().isoformat()}

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8000)"#;

// Rust API Templates
const RUST_CARGO_TOML: &str = r#"[package]
name = "api"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = "0.7"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }"#;

const RUST_MAIN: &str = r#"use axum::{
    routing::get,
    Router,
    Json,
};
use chrono::Utc;
use serde::Serialize;
use std::net::SocketAddr;

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    timestamp: String,
}

#[derive(Serialize)]
struct MessageResponse {
    message: String,
}

async fn root() -> Json<MessageResponse> {
    Json(MessageResponse {
        message: "Hello from Devbox Rust API!".to_string(),
    })
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "OK".to_string(),
        timestamp: Utc::now().to_rfc3339(),
    })
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("API server running on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}"#;

// Go API Templates
const GO_MOD: &str = r#"module api

go 1.21

require github.com/gin-gonic/gin v1.9.1"#;

const GO_MAIN: &str = r#"package main

import (
	"net/http"
	"time"

	"github.com/gin-gonic/gin"
)

type HealthResponse struct {
	Status    string `json:"status"`
	Timestamp string `json:"timestamp"`
}

type MessageResponse struct {
	Message string `json:"message"`
}

func main() {
	router := gin.Default()

	router.GET("/", func(c *gin.Context) {
		c.JSON(http.StatusOK, MessageResponse{
			Message: "Hello from Devbox Go API!",
		})
	})

	router.GET("/health", func(c *gin.Context) {
		c.JSON(http.StatusOK, HealthResponse{
			Status:    "OK",
			Timestamp: time.Now().Format(time.RFC3339),
		})
	})

	router.Run(":9090")
}"#;

// Basic fallback constants
const BASIC_FRONTEND_PACKAGE_JSON: &str = r#"{
  "name": "frontend",
  "version": "1.0.0",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview"
  }
}"#;

const BASIC_FRONTEND_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Devbox Frontend</title>
</head>
<body>
    <h1>Welcome to Devbox!</h1>
    <p>Your frontend service is running.</p>
</body>
</html>"#;

const BASIC_API_PACKAGE_JSON: &str = r#"{
  "name": "api",
  "version": "1.0.0",
  "scripts": {
    "dev": "node server.js",
    "start": "node server.js"
  }
}"#;

const BASIC_API_SERVER_JS: &str = r#"const http = require('http');

const server = http.createServer((req, res) => {
  res.writeHead(200, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify({ message: 'Hello from Devbox API!' }));
});

const PORT = process.env.PORT || 3001;
server.listen(PORT, () => {
  console.log(`API server running on port ${PORT}`);
});"#;

const DATABASE_INIT_SQL: &str = "-- Database initialization script\nCREATE TABLE IF NOT EXISTS users (\n    id SERIAL PRIMARY KEY,\n    name VARCHAR(100),\n    email VARCHAR(100)\n);";

// Service config constants
const NEXTJS_SERVICE_CONFIG: &str = r#"  - name: "frontend"
    service_type: "web"
    command: "cd frontend && npm run dev"
    working_dir: "./frontend"
    health_check:
      type_entry: "http"
      port: 3000
      http_target: "http://localhost:3000"
    dependencies: []"#;

const REACT_SERVICE_CONFIG: &str = r#"  - name: "frontend"
    service_type: "web"
    command: "cd frontend && npm run dev"
    working_dir: "./frontend"
    health_check:
      type_entry: "http"
      port: 5173
      http_target: "http://localhost:5173"
    dependencies: []"#;

const VUE_SERVICE_CONFIG: &str = r#"  - name: "frontend"
    service_type: "web"
    command: "cd frontend && npm run dev"
    working_dir: "./frontend"
    health_check:
      type_entry: "http"
      port: 5173
      http_target: "http://localhost:5173"
    dependencies: []"#;

const SVELTE_SERVICE_CONFIG: &str = r#"  - name: "frontend"
    service_type: "web"
    command: "cd frontend && npm run dev"
    working_dir: "./frontend"
    health_check:
      type_entry: "http"
      port: 5173
      http_target: "http://localhost:5173"
    dependencies: []"#;

const FRONTEND_SERVICE_CONFIG: &str = r#"  - name: "frontend"
    service_type: "web"
    command: "cd frontend && npm run dev"
    working_dir: "./frontend"
    health_check:
      type_entry: "http"
      port: 5173
      http_target: "http://localhost:5173"
    dependencies: []"#;

const NODE_API_SERVICE_CONFIG: &str = r#"  - name: "api"
    service_type: "api"
    command: "cd api && npm run dev"
    working_dir: "./api"
    health_check:
      type_entry: "http"
      port: 3001
      http_target: "http://localhost:3001/health"
    dependencies: []"#;

const PYTHON_API_SERVICE_CONFIG: &str = r#"  - name: "api"
    service_type: "api"
    command: "cd api && python main.py"
    working_dir: "./api"
    health_check:
      type_entry: "http"
      port: 8000
      http_target: "http://localhost:8000/health"
    dependencies: []"#;

const RUST_API_SERVICE_CONFIG: &str = r#"  - name: "api"
    service_type: "api"
    command: "cd api && cargo run"
    working_dir: "./api"
    health_check:
      type_entry: "http"
      port: 8080
      http_target: "http://localhost:8080/health"
    dependencies: []"#;

const GO_API_SERVICE_CONFIG: &str = r#"  - name: "api"
    service_type: "api"
    command: "cd api && go run main.go"
    working_dir: "./api"
    health_check:
      type_entry: "http"
      port: 9090
      http_target: "http://localhost:9090/health"
    dependencies: []"#;

const API_SERVICE_CONFIG: &str = r#"  - name: "api"
    service_type: "api"
    command: "cd api && npm run dev"
    working_dir: "./api"
    health_check:
      type_entry: "http"
      port: 3001
      http_target: "http://localhost:3001"
    dependencies: []"#;

const DATABASE_SERVICE_CONFIG: &str = r#"  - name: "database"
    service_type: "database"
    command: "docker run -p 5432:5432 -e POSTGRES_PASSWORD=devbox postgres:15"
    working_dir: "./database"
    health_check:
      type_entry: "port"
      port: 5432
      http_target: ""
    dependencies: []"#;

const CACHE_SERVICE_CONFIG: &str = r#"  - name: "cache"
    service_type: "cache"
    command: "docker run -p 6379:6379 redis:7-alpine"
    working_dir: "./cache"
    health_check:
      type_entry: "port"
      port: 6379
      http_target: ""
    dependencies: []"#;

const AUTH_SERVICE_CONFIG: &str = r#"  - name: "auth"
    service_type: "api"
    command: "echo 'Auth service starting'"
    working_dir: "./auth"
    health_check:
      type_entry: "none"
    dependencies: ["database"]"#;

const GENERIC_SERVICE_CONFIG: &str = r#"  - name: "generic"
    service_type: "service"
    command: "echo 'Service starting'"
    working_dir: "./generic"
    health_check:
      type_entry: "none"
    dependencies: []"#;

// Docker constants
const DOCKERFILE_NEXTJS: &str = r#"FROM node:18-alpine
WORKDIR /app
COPY package*.json ./
RUN npm install
COPY . .
EXPOSE 3000
CMD ["npm", "run", "dev"]"#;

const DOCKERFILE_FRONTEND: &str = r#"FROM node:18-alpine
WORKDIR /app
COPY package*.json ./
RUN npm install
COPY . .
EXPOSE 5173
CMD ["npm", "run", "dev"]"#;

const DOCKERFILE_API: &str = r#"FROM node:18-alpine
WORKDIR /app
COPY package*.json ./
RUN npm install
COPY . .
EXPOSE 3001
CMD ["npm", "start"]"#;

const DOCKER_COMPOSE: &str = r#"version: '3.8'
services:
  frontend:
    build:
      context: .
      dockerfile: docker/Dockerfile.frontend
    ports:
      - "3000:3000"
    volumes:
      - ./frontend:/app
      - /app/node_modules

  api:
    build:
      context: .
      dockerfile: docker/Dockerfile.api
    ports:
      - "3001:3001"
    volumes:
      - ./api:/app
      - /app/node_modules

  database:
    image: postgres:15
    environment:
      POSTGRES_PASSWORD: devbox
      POSTGRES_DB: devbox
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data

volumes:
  postgres_data:"#;

const DOCKER_IGNORE: &str = r#"node_modules
npm-debug.log
.git
.env
.next"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_validate_project_name() {
        let args = InitArgs {
            name: None,
            yes: false,
            template: None,
            docker: false,
        };

        assert!(args.validate_project_name("my-project").is_ok());
        assert!(args.validate_project_name("project123").is_ok());
        assert!(args.validate_project_name("my_project").is_ok());
        assert!(args.validate_project_name("").is_ok());
        assert!(args.validate_project_name("my project").is_err());
        assert!(args.validate_project_name("my@project").is_err());
    }

    #[tokio::test]
    async fn test_service_selection() {
        let args = InitArgs {
            name: None,
            yes: true,
            template: None,
            docker: false,
        };

        let test_cases = vec![
            ("nextjs", vec!["frontend"]),
            ("react", vec!["frontend"]),
            ("node", vec!["api"]),
            ("python", vec!["api"]),
            ("fullstack", vec!["frontend", "api", "database"]),
        ];

        for (template, expected_services) in test_cases {
            let services = args.select_services(template).await.unwrap();
            assert_eq!(services, expected_services, "Failed for template: {}", template);
        }
    }

    #[tokio::test]
    async fn test_devbox_yaml_generation() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let args = InitArgs {
            name: Some("test-yaml".to_string()),
            yes: true,
            template: Some("nextjs".to_string()),
            docker: false,
        };

        std::fs::create_dir_all("test-yaml/frontend").unwrap();
        
        let result = args.generate_devbox_yaml("test-yaml", "nextjs", &["frontend".to_string()], false).await;
        assert!(result.is_ok());

        let yaml_content = fs::read_to_string("test-yaml/devbox.yaml").unwrap();
        assert!(yaml_content.contains("name: \"test-yaml\""));
        assert!(yaml_content.contains("nextjs project"));
        assert!(yaml_content.contains("nodejs@latest"));
    }
}