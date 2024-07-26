/// This macro is used to notify sp1 cycle tracker that a new routine is started.
#[macro_export]
macro_rules! cycle_tracker_start {
    ($($arg:tt)*) => {
        #[cfg(all(feature = "sp1", feature = "cycle-tracker"))]
        println!("cycle-tracker-start: {}", format!($($arg)*));
    };
}

/// This macro is used to notify sp1 cycle tracker that a routine is ended.
#[macro_export]
macro_rules! cycle_tracker_end {
    ($($arg:tt)*) => {
        #[cfg(all(feature = "sp1", feature = "cycle-tracker"))]
        println!("cycle-tracker-end: {}", format!($($arg)*));
    };
}
