use core::sync::atomic::{AtomicBool, AtomicI32};

/// Expression trigger from the web UI
pub static EXPRESSION: AtomicI32 = AtomicI32::new(0);

/// Set to true after WiFi config is saved; the main loop then triggers a
/// soft-reset
pub static RESTART_REQUESTED: AtomicBool = AtomicBool::new(false);
