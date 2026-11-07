use std::process::ExitCode;

fn main() -> ExitCode {
    unsafe { taskmaster::entrypoint(std::env::args().skip(1).collect()) }
}
