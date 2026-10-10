#[doc(hidden)]
pub use time;

fn add_error_sources(mut dst: String, mut maybe_source: Option<&dyn core::error::Error>) -> String {
    while let Some(source) = maybe_source.take() {
        let error_msg = source.to_string().replace("\n", "\n\t\t");
        dst += &format!("\n\toccured because of: {error_msg}");
        maybe_source = source.source();
    }
    dst
}

pub fn log(error: &dyn core::error::Error, log_level: LogLevel) {
    use crate as logging;

    let str = add_error_sources(error.to_string(), error.source());
    match log_level {
        LogLevel::Debug => debug!("{str}"),
        LogLevel::Error => error!("{str}"),
        LogLevel::Warning => warning!("{str}"),
        LogLevel::Info => info!("{str}"),
    }
}

pub fn log_with_msg(
    error_msg: impl Into<String>,
    source: &dyn core::error::Error,
    log_level: LogLevel,
) {
    use crate as logging;

    let str = add_error_sources(error_msg.into(), Some(source));
    match log_level {
        LogLevel::Debug => debug!("{str}"),
        LogLevel::Error => error!("{str}"),
        LogLevel::Warning => warning!("{str}"),
        LogLevel::Info => info!("{str}"),
    }
}

pub enum LogLevel {
    Debug,
    Error,
    Warning,
    Info,
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {{
        println!("{} Debug: {}", logging::time::get_current_time(), &format!($($arg)*));
    }};
}

#[macro_export]
macro_rules! warning {
    ($($arg:tt)*) => {{
        println!("{} Warning: {}", logging::time::get_current_time(), &format!($($arg)*));
    }};
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        println!("{} Info: {}", logging::time::get_current_time(), &format!($($arg)*));
    }};
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {{
        eprintln!("{} Error: {}", logging::time::get_current_time(), &format!($($arg)*));
    }};
}
