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
        #[cfg(feature = "dev")]
        {
            trace!($($arg)*);
        }
    };
}

/// This macro is for logging level info
#[macro_export]
macro_rules! dev_info {
    ($($arg:tt)*) => {
        #[cfg(feature = "dev")]
        {
            info!($($arg)*);
        }
    };
}

/// This macro is for logging level error
#[macro_export]
macro_rules! dev_error {
    ($($arg:tt)*) => {
        #[cfg(feature = "dev")]
        {
            error!($($arg)*);
        }
    };
}

/// This macro is for logging level debug
#[macro_export]
macro_rules! dev_debug {
    ($($arg:tt)*) => {
        #[cfg(feature = "dev")]
        {
            debug!($($arg)*);
        }
    };
}

/// This macro is for logging level warn
#[macro_export]
macro_rules! dev_warn {
    ($($arg:tt)*) => {
        #[cfg(feature = "dev")]
        {
            warn!($($arg)*);
        }
    };
}

/// This macro is for measuring duration to metrics
#[macro_export]
macro_rules! measure_duration_histogram {
    ($label:ident, $e:expr) => {{
        #[cfg(feature = "metrics")]
        let _start = std::time::Instant::now();

        #[allow(clippy::let_and_return)]
        let _result = $e;

        #[cfg(feature = "metrics")]
        $crate::metrics::REGISTRY
            .$label
            .observe(_start.elapsed().as_millis() as f64);

        dev_debug!(
            "measured duration {} = {:?}",
            stringify!($label),
            _start.elapsed()
        );

        _result
    }};
}
