macro_rules! info {
    ($fmt:expr) => (println!(concat!("::: ", $fmt)));
    ($fmt:expr, $($arg:tt)*) => (println!(concat!("::: ", $fmt), $($arg)*));
}

pub(crate) use info;
