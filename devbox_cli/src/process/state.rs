use std::collections::HashMap;
use std::process::Child;
use serde::{Serialize, Deserialize};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub service_name: String,
    pub project_name: String,
    pub command: String,
    pub start_time: SystemTime,
    pub status: ProcessStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessStatus {
    Running,
    Stopped,
    Error(String),
}

#[derive(Debug)]
pub struct ProcessState {
    processes: HashMap<u32, ProcessInfo>,
    state_file: std::path::PathBuf,
}

impl ProcessState {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let state_dir = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("devbox");
        
        std::fs::create_dir_all(&state_dir)?;
        
        let state_file = state_dir.join("processes.json");
        let mut state = ProcessState {
            processes: HashMap::new(),
            state_file,
        };
        
        // Load existing state if available
        state.load_state()?;
        
        Ok(state)
    }
    
    pub fn add_process(&mut self, child: &mut Child, service_name: &str, project_name: &str, command: &str) -> Result<(), Box<dyn std::error::Error>> {
        let pid = child.id();
        let process_info = ProcessInfo {
            pid,
            service_name: service_name.to_string(),
            project_name: project_name.to_string(),
            command: command.to_string(),
            start_time: SystemTime::now(),
            status: ProcessStatus::Running,
        };
        
        self.processes.insert(pid, process_info);
        self.save_state()?;
        
        Ok(())
    }
    
    pub fn remove_process(&mut self, pid: u32) -> Result<(), Box<dyn std::error::Error>> {
        self.processes.remove(&pid);
        self.save_state()?;
        Ok(())
    }
    
    pub fn get_project_processes(&self, project_name: &str) -> Vec<&ProcessInfo> {
        self.processes.values()
            .filter(|p| p.project_name == project_name && matches!(p.status, ProcessStatus::Running))
            .collect()
    }
    
    pub fn get_all_processes(&self) -> Vec<&ProcessInfo> {
        self.processes.values().collect()
    }
    
    fn save_state(&self) -> Result<(), Box<dyn std::error::Error>> {
        let serialized = serde_json::to_string_pretty(&self.processes)?;
        std::fs::write(&self.state_file, serialized)?;
        Ok(())
    }
    
    fn load_state(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.state_file.exists() {
            let content = std::fs::read_to_string(&self.state_file)?;
            self.processes = serde_json::from_str(&content)?;
        }
        Ok(())
    }
}