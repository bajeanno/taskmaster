#[allow(dead_code)]
pub mod time;

pub enum LogLevel {
    Debug,
    Error,
    Warning,
    Info,
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {{
        use logging::time;
        println!("{} Debug: {}", time::get_current_time(), &format!($($arg)*));
    }};
}

#[macro_export]
macro_rules! warning {
    ($($arg:tt)*) => {{
        use logging::time;
        println!("{} Warning: {}", time::get_current_time(), &format!($($arg)*));
    }};
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        use logging::time;
        println!("{} Info: {}", time::get_current_time(), &format!($($arg)*));
    }};
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {{
        use logging::time;
        eprintln!("{} Error: {}", time::get_current_time(), &format!($($arg)*));
    }};
}
