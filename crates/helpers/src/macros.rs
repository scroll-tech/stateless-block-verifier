/// This macro is used to notify sp1 cycle tracker that a new routine has started.
#[macro_export]
macro_rules! cycle_track {
    ($e:expr, $($arg:tt)*) => {
        {
            #[cfg(all(feature = "sp1", feature = "cycle-tracker"))]
            println!("cycle-tracker-start: {}", format!($($arg)*));

            #[allow(clippy::let_and_return)]
            let __cycle_track_result = $e;

            #[cfg(all(feature = "sp1", feature = "cycle-tracker"))]
            println!("cycle-tracker-end: {}", format!($($arg)*));

            __cycle_track_result
        }
    };
}

/// This macro is used to notify sp1 cycle tracker that a new routine has started.
#[macro_export]
macro_rules! cycle_tracker_start {
    ($($arg:tt)*) => {
        #[cfg(all(feature = "sp1", feature = "cycle-tracker"))]
        println!("cycle-tracker-start: {}", format!($($arg)*));
    };
}

/// This macro is used to notify sp1 cycle tracker that a routine has ended.
#[macro_export]
macro_rules! cycle_tracker_end {
    ($($arg:tt)*) => {
        #[cfg(all(feature = "sp1", feature = "cycle-tracker"))]
        println!("cycle-tracker-end: {}", format!($($arg)*));
    };
}

/// This macro is for logging level trace
#[macro_export]
macro_rules! dev_trace {
    ($($arg:tt)*) => {
        {
            #[cfg(any(feature = "dev", test))]
            $crate::tracing::trace!($($arg)*);
        }
    };
}

/// This macro is for logging level info
#[macro_export]
macro_rules! dev_info {
    ($($arg:tt)*) => {
        {
            #[cfg(any(feature = "dev", test))]
            $crate::tracing::info!($($arg)*);
        }
    };
}

/// This macro is for logging level error
#[macro_export]
macro_rules! dev_error {
    ($($arg:tt)*) => {
        {
            #[cfg(any(feature = "dev", test))]
            $crate::tracing::error!($($arg)*);
        }
    };
}

/// This macro is for logging level debug
#[macro_export]
macro_rules! dev_debug {
    ($($arg:tt)*) => {
        {
            #[cfg(any(feature = "dev", test))]
            $crate::tracing::debug!($($arg)*);
        }
    };
}

/// This macro is for logging level warn
#[macro_export]
macro_rules! dev_warn {
    ($($arg:tt)*) => {
        {
            #[cfg(any(feature = "dev", test))]
            $crate::tracing::warn!($($arg)*);
        }
    };
}

/// This macro is used to manually drop an expression on zkvm (non x86/aarch64 targets).
#[macro_export]
#[cfg(not(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")))]
macro_rules! manually_drop_on_zkvm {
    ($e:expr) => {
        std::mem::ManuallyDrop::new($e)
    };
}

/// This macro is used to manually drop an expression on zkvm (non x86/aarch64 targets).
#[macro_export]
#[cfg(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64"))]
macro_rules! manually_drop_on_zkvm {
    ($e:expr) => {
        $e
    };
}
