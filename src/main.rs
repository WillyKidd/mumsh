use std::sync::Arc;

use linefeed::{Interface, ReadResult, Command};
use libc::{signal, SIGTSTP, SIGTTOU, SIG_IGN};

mod executor;
mod input;
mod parser;
mod types;
mod mumsh;
mod common;
mod builtin;

fn main() {
    unsafe {
        signal(SIGTSTP, SIG_IGN);
        signal(SIGTTOU, SIG_IGN);
    }
    let reader = match Interface::new("mumsh") {
        Ok(x) => x,
        Err(e) => {
            println!("linefeed error {}", e);
            return;
        }
    };
    reader.define_function("input-check", Arc::new(input::InputCheck));
    reader.bind_sequence("\r", Command::from_str("input-check"));

    let mut sh = mumsh::Mumsh::new();
    loop {
        match reader.set_prompt("mumsh $ ") {
            Ok(_) => {},
            Err(_) => {eprintln!("linefeed: error setting prompt")},
        }
        match reader.read_line() {
            Ok(ReadResult::Input(mut line)) => {
                line = input::remove_multiline_prompt(&line);
                if line.trim() == "exit" {
                    println!("bye~");
                    return;
                }
                executor::run(&line, &mut sh);
            },
            Ok(ReadResult::Signal(_)) => {
                println!("received signal");
            },
            Ok(ReadResult::Eof) => {
                println!("bye~");
                break;
            },
            Err(e) => {
                eprintln!("\nmumsh: parse error near `{}\'", e);
                match reader.set_buffer("") {
                    Ok(_) => {},
                    Err(_) => {eprintln!("linefeed: error setting buffer")},
                }
                continue;
            }
        };
    }

}
