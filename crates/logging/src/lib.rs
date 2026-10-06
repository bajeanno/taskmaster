pub mod logging;
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

pub fn print_error(error: &dyn core::error::Error) {
    let str = format_error(error);
    error!("{str}");
}

pub fn print_warning(error: &dyn core::error::Error) {
    let str = format_error(error);
    warning!("{str}");
}

pub fn print_debug(error: &dyn core::error::Error) {
    let str = format_error(error);
    debug!("{str}");
}