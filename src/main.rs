use std::process::ExitCode;

fn main() -> ExitCode {
    unsafe { taskmaster::main(None) }
}
