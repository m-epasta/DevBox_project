use crate::process::state::{ProcessState, ProcessInfo};
use crate::process::global::get_global_state;
use std::sync::{Mutex, OnceLock};

pub struct ProcessManager; // No state field - stateless

impl ProcessManager {
    pub fn get_running_services() -> Vec<ProcessInfo> {
        let state = get_global_state(); // Lock inside method
        state.get_all_processes()
            .iter()
            .map(|running_process| {
                ProcessInfo {
                    pid: running_process.info.pid,
                    service_name: running_process.info.service_name.clone(),
                    project_name: running_process.info.project_name.clone(),
                    command: running_process.info.command.clone(),
                    start_time: running_process.info.start_time,
                    status: running_process.info.status.clone(),
                }
            })
            .collect()
    }

    pub fn find_service(service_name: &str) -> Option<ProcessInfo> { 
        for service in Self::get_running_services() {
            if service.service_name == service_name {
                return Some(service)
            }
        }
        None
    }

    pub fn is_service_running(service_name: &str) -> bool {
        Self::find_service(service_name).is_some()
    }
}