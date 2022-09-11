use std::sync::Arc;

use linefeed::{Interface, ReadResult, Command};
use libc::{signal, SIGINT, SIGQUIT, SIGTSTP, SIGTTIN, SIGTTOU, SIG_IGN};
use nix::{unistd::{isatty, tcgetpgrp, getpgrp, Pid, getpid, setpgid}, sys::signal::kill};

mod executor;
mod input;
mod parser;
mod types;
mod mumsh;
mod common;
mod builtin;

fn main() {
    let shell_is_interactive;
    let mut shell_pgid = getpgrp();
    match isatty(1) {
        Ok(x) => shell_is_interactive = x,
        Err(e) => {
            eprintln!("{}", e);
            return;
        }
    };
    if shell_is_interactive {
        // loop until mumsh is in foreground
        while tcgetpgrp(1).unwrap() != shell_pgid {
            kill(Pid::from_raw(-(shell_pgid.as_raw())), nix::sys::signal::SIGTTIN).unwrap();
        }
        // ignore iteractive and job-control signals
        unsafe {
            signal (SIGINT, SIG_IGN);
            signal (SIGQUIT, SIG_IGN);
            signal (SIGTSTP, SIG_IGN);
            signal (SIGTTIN, SIG_IGN);
            signal (SIGTTOU, SIG_IGN);
            // signal (SIGCHLD, SIG_IGN);
        }
        // put mumsh in her own process group
        shell_pgid = getpid();
        match setpgid(shell_pgid, shell_pgid) {
            Ok(_) => {},
            Err(e) => {
                eprintln!("setpgid: {}", e);
                return;
            }
        };
        mumsh::Mumsh::set_foreground_pg(shell_pgid.as_raw());
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

}
