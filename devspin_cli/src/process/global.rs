use std::sync::Mutex;
use once_cell::sync::Lazy;
use super::state::ProcessState;

pub static GLOBAL_STATE: Lazy<Mutex<ProcessState>> = Lazy::new(|| {
    Mutex::new(ProcessState::new())
});

pub fn get_global_state() -> std::sync::MutexGuard<'static, ProcessState> {
    GLOBAL_STATE.lock().unwrap()
}