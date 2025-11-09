use std::sync::Mutex;
use once_cell::sync::Lazy;
use super::state::ProcessState;

static GLOBAL_STATE: Lazy<Mutex<ProcessState>> = Lazy::new(|| {
    println!("🔍 DEBUG: Initializing global state");
    Mutex::new(ProcessState::new())
});

pub fn get_global_state() -> std::sync::MutexGuard<'static, ProcessState> {
    println!("🔍 DEBUG: Getting global state lock");
    GLOBAL_STATE.lock().unwrap()
}