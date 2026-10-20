use std::process::ExitCode;

fn main() -> ExitCode {
    unsafe { taskmaster::main(std::env::args().skip(1).collect()) }
}
