use std::collections::HashMap;
use std::process::Child;

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub service_name: String,
    pub project_name: String,
    pub command: String,
    pub start_time: std::time::SystemTime,
    pub status: ProcessStatus,
}

#[derive(Debug, Clone)]
pub enum ProcessStatus {
    Running,
    Stopped,
    Error(String),
}

#[derive(Debug)]
pub struct ProcessState {
    processes: HashMap<u32, ProcessInfo>,
}

impl ProcessState {
    pub fn new() -> Self {
        ProcessState {
            processes: HashMap::new(),
        }
    }
    
    pub fn add_process(&mut self, child: &mut Child, service_name: &str, project_name: &str, command: &str) -> Result<(), Box<dyn std::error::Error>> {
        let pid = child.id();
        let process_info = ProcessInfo {
            pid,
            service_name: service_name.to_string(),
            project_name: project_name.to_string(),
            command: command.to_string(),
            start_time: std::time::SystemTime::now(),
            status: ProcessStatus::Running,
        };
        
        self.processes.insert(pid, process_info);
        Ok(())
    }
    
    pub fn remove_process(&mut self, pid: u32) -> Result<(), Box<dyn std::error::Error>> {
        self.processes.remove(&pid);
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
    
    // 🆕 Get process count for statistics
    pub fn process_count(&self) -> usize {
        self.processes.len()
    }
    
    // 🆕 Check if a specific service is running
    pub fn is_service_running(&self, project_name: &str, service_name: &str) -> bool {
        self.processes.values()
            .any(|p| p.project_name == project_name && p.service_name == service_name && matches!(p.status, ProcessStatus::Running))
    }

}
impl Default for ProcessState {
        fn default() -> Self {
                Self::new()
        }
}

impl Drop for ProcessState {
    fn drop(&mut self) {
        if !self.processes.is_empty() {
            eprintln!("⚠️  Warning: {} processes still running", self.processes.len());
        }
    }
}