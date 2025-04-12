use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
static ENABLED: AtomicBool = AtomicBool::new(true);

pub fn disable() {
    ENABLED.store(false, SeqCst);
}

macro_rules! info {
    ($fmt:expr) => (crate::status::println(format!($fmt)));
    ($fmt:expr, $($arg:tt)*) => (crate::status::println(format!($fmt, $($arg)*)));
}

pub fn println(message: String) {
    if ENABLED.load(SeqCst) {
        println!("::: {message}");
    }
}

pub(crate) use info;
