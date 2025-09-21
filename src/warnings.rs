use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, OnceLock};

/// Global warning state using thread-safe primitives
static WARNING_STATE: OnceLock<Arc<Mutex<WarningState>>> = OnceLock::new();

#[derive(Debug)]
struct WarningState {
    maximum: u32,
    raised: HashMap<String, u32>,
    muted: HashSet<String>,
}

impl Default for WarningState {
    fn default() -> Self {
        Self {
            maximum: 10,
            raised: HashMap::new(),
            muted: HashSet::new(),
        }
    }
}

/// Get or initialize the global warning state
fn get_state() -> Arc<Mutex<WarningState>> {
    WARNING_STATE.get_or_init(|| Arc::new(Mutex::new(WarningState::default()))).clone()
}

/// Set the maximum number of warnings for a given type
pub fn set_warnings_maximum(maximum: u32) {
    let state = get_state();
    if let Ok(mut state) = state.lock() {
        state.maximum = maximum;
    }
}

/// Get the current warnings maximum
pub fn get_warnings_maximum() -> u32 {
    let state = get_state();
    state.lock().map(|s| s.maximum).unwrap_or(10)
}

/// Add a warning type to the muted set
pub fn mute_warning(name: impl Into<String>) {
    let state = get_state();
    if let Ok(mut state) = state.lock() {
        state.muted.insert(name.into());
    }
}

/// Check if a warning is muted
pub fn is_warning_muted(name: &str) -> bool {
    let state = get_state();
    state.lock()
        .map(|s| s.muted.contains(name))
        .unwrap_or(false)
}

/// Record a warning being raised
pub fn raise_warning(name: impl Into<String>) -> bool {
    let name = name.into();
    let state = get_state();

    if let Ok(mut state) = state.lock() {
        if state.muted.contains(&name) {
            return false; // Warning is muted
        }

        let count = state.raised.entry(name.clone()).or_insert(0);
        *count += 1;

        // Return true if we should show this warning (not exceeded maximum)
        *count <= state.maximum
    } else {
        true // Show warning if we can't get the lock
    }
}

/// Check if a warning has exceeded the maximum
pub fn has_exceeded_maximum(name: &str) -> bool {
    let state = get_state();
    state.lock()
        .map(|s| {
            let count = s.raised.get(name).copied().unwrap_or(0);
            count > s.maximum
        })
        .unwrap_or(false)
}

/// Get the count of times a warning was raised
pub fn get_warning_count(name: &str) -> u32 {
    let state = get_state();
    state.lock()
        .map(|s| s.raised.get(name).copied().unwrap_or(0))
        .unwrap_or(0)
}

/// Get a summary of all warnings that exceeded the maximum
pub fn get_warning_summary() -> Vec<(String, u32, u32)> {
    let state = get_state();
    if let Ok(state) = state.lock() {
        state.raised.iter()
            .filter(|(name, count)| {
                !state.muted.contains(*name) && **count > state.maximum
            })
            .map(|(name, count)| {
                let excess = *count - state.maximum;
                (name.clone(), *count, excess)
            })
            .collect()
    } else {
        Vec::new()
    }
}

/// Clear all warning state
pub fn clear_warnings() {
    let state = get_state();
    if let Ok(mut state) = state.lock() {
        state.raised.clear();
        state.muted.clear();
    }
}