use devbox_cli::configs::yaml_parser::ProjectConfig;
use devbox_cli::cli::start::StartArgs;
use devbox_cli::process::{ProcessState, ProcessInfo, ProcessStatus};
use std::process::Command;
use std::time::SystemTime;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_load_valid_config() {
        let yaml_content = r#"
        name: "test-app"
        description: "test: should succeed"
        commands:
            start:
                dev: "npm run dev"        
                build: "npm run build"    
        "#;
        let config: ProjectConfig = serde_yaml::from_str(yaml_content).unwrap();

        assert_eq!(config.name, "test-app");
        assert_eq!(config.commands.start.dev, "npm run dev")
    }

    #[test]
    fn test_invalid_config_fails() {
        let invalid_yaml = "name: 123";

        let result = serde_yaml::from_str::<ProjectConfig>(invalid_yaml);
        assert!(result.is_err());
        // TODO: try more errors that is possible
    }

    #[tokio::test]
    async fn test_start_command_dry_run() {
        let args = StartArgs {
            name: "tests/fixtures/commands_test.rs".to_string(),  
            env: None,
            verbose: false,
            background: false,
            dry_run: true,
            only: None,
            skip: None,
        };
        
        println!("Testing with project name: {}", args.name);
        println!("Looking for file: {}/devbox.yaml", args.name);

        let result = args.handle().await;
        assert!(result.is_ok(), "Dry run should succeed");
    }

    #[tokio::test]
    async fn test_start_command_with_filters() {
        let args = StartArgs {
            name: "tests/fixtures/test-project".to_string(), 
            env: None,
            verbose: true,
            background: false,
            dry_run: true,
            only: Some(vec!["frontend".to_string()]),
            skip: Some(vec!["database".to_string()]),
        };
        
        let result = args.handle().await;
        assert!(result.is_ok(), "Filtered dry run should succeed");
    }

    #[tokio::test]
    async fn test_start_background_command() {
        let args = StartArgs {
            name: "tests/fixtures/background-project".to_string(),  
            env: None,
            verbose: true,
            background: true,  
            dry_run: true,     
            only: Some(vec!["frontend".to_string()]),
            skip: Some(vec!["database".to_string()]),
        };
        
        let result = args.handle().await;
        assert!(result.is_ok(), "Background dry run should succeed");
    }

    #[tokio::test]
    async fn test_start_verbose_command() {
        let args = StartArgs {
            name: "tests/fixtures/verbose-project".to_string(),
            env: None,
            verbose: true,  
            background: false,
            dry_run: true,
            only: None,
            skip: None,
        };
        
        println!("Testing VERBOSE mode...");
        let result = args.handle().await;
        assert!(result.is_ok(), "Verbose dry run should succeed");
        
        // The test should show extra details in output due to verbose mode
    } 
    
    #[tokio::test]
    async fn test_start_non_verbose_command() {
        let args = StartArgs {
            name: "tests/fixtures/verbose-project".to_string(), 
            env: None,
            verbose: false,  // ← No verbose
            background: false,
            dry_run: true,
            only: None,
            skip: None,
        };
        
        println!("Testing NON-VERBOSE mode...");
        let result = args.handle().await;
        assert!(result.is_ok(), "Non-verbose dry run should succeed");
        
        // Should show minimal output
    }  

    #[test]
    fn test_process_creation() {
        let result = ProcessState::new();

        assert!(result.is_ok(), "Process State should be created successfully");

        let _process_state = result.unwrap();
    }

    #[test]
    fn test_process_state_operations() {
        let mut process_state = ProcessState::new().unwrap();

        let processes = process_state.get_all_processes();
        assert_eq!(processes.len(), 0, "New processes should have no processes");

        let mut child = Command::new("sleep")
            .arg("1")
            .spawn()
            .expect("Failed to spawn test process");

        let add_result = process_state.add_process(
            &mut child,
            "test-service",
            "test-project",
            "sleep 1"
        );

        assert!(add_result.is_ok(), "Should have added process succesfully");

        let processes = process_state.get_all_processes();
        assert_eq!(processes.len(), 1, "Should have one process after adding");

        let project_processes = process_state.get_project_processes("test-project");
        assert_eq!(processes.len(), 1, "Should have one process after adding");

        let pid = child.id();
        let remove_result = process_state.remove_process(pid);
        assert!(remove_result.is_ok(), "Should remove process successfully");

        let processes = process_state.get_all_processes();
        assert_eq!(processes.len(), 0, "Should have no processes after removal");

        let _ = child.kill();

    }
    
    #[test]
    fn test_process_state_persistance() {
        let mut process_state1 = ProcessState::new().unwrap();

        let mut child = Command::new("sleep")
            .arg("1")
            .spawn()
            .expect("Failed to spawn test process");

        let _ = process_state1.add_process(&mut child, 
            "persistance-service",
            "persistance-project",
            "sleep 1");
        
        let pid = child.id();

        let process_state2 = ProcessState::new().unwrap();
        let processes = process_state2.get_all_processes();

        assert_eq!(processes.len(), 1, "Should have loaded persisted process");
        assert_eq!(processes[0].pid, pid, "Should have same PID");
        assert_eq!(processes[0].service_name, "persistance-service");
        assert_eq!(processes[0].project_name, "persistance-project");

        let mut process_state3 = ProcessState::new().unwrap();
        process_state3.remove_process(pid).unwrap();
        let _ = child.kill();
    }

    #[test]
    fn test_process_state_error_cases() {
        let mut process_state = ProcessState::new().unwrap();

        let remove_result = process_state.remove_process(99999);
        assert!(remove_result.is_ok(), "Removing non-existent PID should not crash");

        let processes = process_state.get_project_processes("non-existent-project");
        assert_eq!(processes.len(), 0, "Should not return empty Vec for non-existent project");
    }

    #[tokio::test]
    async fn test_start_command_tracks_processes() {
        let args = StartArgs {
            name: "tests/fixtures/verbose-project".to_string(),
            env: None,
            verbose: false,
            background: false,
            dry_run: true,
            only: None,
            skip: None,
        };

        let result = args.handle().await;
        assert!(result.is_ok(), "Dry run should succeed");

        let process_state = ProcessState::new();
        assert!(process_state.is_ok(), "Process state should work with Start Args");

    }
}
