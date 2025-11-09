// src/process/state.rs
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
pub struct RunningProcess {
    pub info: ProcessInfo,
    pub child: Child,
}

#[derive(Debug)]
pub struct ProcessState {
    processes: HashMap<u32, RunningProcess>,
}

impl ProcessState {
    pub fn new() -> Self {
        ProcessState {
            processes: HashMap::new(),
        }
    }

    pub fn add_process(
        &mut self,
        child: Child,
        service_name: &str,
        project_name: &str,
        command: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pid = child.id();
        
        println!("🔍 DEBUG add_process: Starting - PID: {}, Service: {}", pid, service_name);
        println!("🔍 DEBUG add_process: Current processes count: {}", self.processes.len());

        let process_info = ProcessInfo {
            pid,
            service_name: service_name.to_string(),
            project_name: project_name.to_string(),
            command: command.to_string(),
            start_time: std::time::SystemTime::now(),
            status: ProcessStatus::Running,
        };

        println!("🔍 DEBUG add_process: Created ProcessInfo, inserting into HashMap...");

        self.processes.insert(
            pid,
            RunningProcess {
                info: process_info,
                child,
            },
        );

        println!("🔍 DEBUG add_process: After insert - processes count: {}", self.processes.len());
        println!("🔍 DEBUG add_process: Successfully added process {}", pid);

        Ok(())
    }

    pub fn get_project_processes(&mut self, project_name: &str) -> Vec<&mut RunningProcess> {
        self.processes
            .values_mut()
            .filter(|p| {
                p.info.project_name == project_name
                    && matches!(p.info.status, ProcessStatus::Running)
            })
            .collect()
    }

    pub fn get_all_processes(&self) -> &HashMap<u32, RunningProcess> {
        &self.processes
    }

    pub fn remove_process(&mut self, pid: u32) -> Result<(), Box<dyn std::error::Error>> {
        self.processes.remove(&pid);
        Ok(())
    }

    pub fn process_count(&self) -> usize {
        self.processes.len()
    }

    pub fn is_service_running(&self, project_name: &str, service_name: &str) -> bool {
        self.processes.values().any(|p| {
            p.info.project_name == project_name
                && p.info.service_name == service_name
                && matches!(p.info.status, ProcessStatus::Running)
        })
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
            eprintln!(
                "⚠️  Warning: {} processes still running",
                self.processes.len()
            );

            // FIX: Use iter_mut() and take ownership in the loop
            let processes = std::mem::take(&mut self.processes);

            for (_, mut running_process) in processes.into_iter() {
                // Now we can mutate because we own running_process
                let _ = running_process.child.kill();
                let _ = running_process.child.wait();
            }
        }
    }
}
