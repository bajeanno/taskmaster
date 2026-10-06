pub mod logging;
pub use logging::LogLevel;
pub use logging::time;

fn format_error(error: &dyn core::error::Error) -> String {
    let mut str = String::new();
    str += &error.to_string();

    let mut maybe_source = error.source();
    while let Some(source) = maybe_source.take() {
        let error_msg = source.to_string().replace("\n", "\n\t\t");
        str += &format!("\n\toccured because of: {error_msg}");
        maybe_source = source.source();
    }
    str
}

pub fn print_log(error: &dyn core::error::Error, log_level: LogLevel) {
    let str = format_error(error);
    match log_level {
        LogLevel::Debug => debug!("{str}"),
        LogLevel::Error => error!("{str}"),
        LogLevel::Warning => warning!("{str}"),
        LogLevel::Info => info!("{str}"),
    }
}
