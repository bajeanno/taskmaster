mod client;
mod shell;

use std::env::args;

use client::{parsing::parse_command, send_command};

fn main() {
    if let Some(_) = args().nth(1) {
        let args = {
            args().fold(Vec::new(), |mut acc: Vec<String>, value: String| {
                acc.push(value.clone());
                acc
            })
        };
        let command = parse_command(args);
        send_command(command);
    } else {
        shell::run();
    }
}
