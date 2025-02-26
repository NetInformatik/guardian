use std::sync::atomic::AtomicBool;

// Global status flags for Guardian System
// Peripheral Device (PD) status
pub static PD_ONLINE: AtomicBool = AtomicBool::new(false);
