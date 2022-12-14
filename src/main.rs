use std::sync::Arc;

use colored::{self, Colorize};
use linefeed::{Interface, ReadResult, Command};
use libc;
use nix::{unistd::{isatty, tcgetpgrp, getpgrp, Pid, getpid, setpgid}, sys::signal::kill};
use termios::{*, os::linux::ECHOCTL};

mod executor;
mod input;
mod parser;
mod types;
mod mumsh;
mod common;
mod builtin;

fn main() {
    let mut sh = mumsh::Mumsh::new();
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
            libc::signal(libc::SIGINT, libc::SIG_IGN);
            libc::signal(libc::SIGQUIT,libc::SIG_IGN);
            libc::signal(libc::SIGTSTP,libc::SIG_IGN);
            libc::signal(libc::SIGTTIN,libc::SIG_IGN);
            libc::signal(libc::SIGTTOU,libc::SIG_IGN);
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
        sh.set_foreground_pg(shell_pgid.as_raw());
        let reader = match Interface::new("mumsh") {
            Ok(x) => x,
            Err(e) => {
                println!("linefeed error {}", e);
                return;
            }
        };
        reader.define_function("input-check", Arc::new(input::InputCheck));
        reader.bind_sequence("\r", Command::from_str("input-check"));

        let mut prompt = " mumsh $ ".on_truecolor(10, 122, 60).truecolor(255, 255, 255).bold().to_string();
        prompt.push_str(&"".truecolor(10, 122, 60).bold().to_string());
        prompt.push(' ');

        let mut attr = Termios::from_fd(0).unwrap();
        attr.c_lflag &= !ECHOCTL;
        tcsetattr(0, TCSANOW, &mut attr).unwrap();

        loop {
            match reader.set_prompt(&prompt) {
                Ok(_) => {},
                Err(_) => {eprintln!("linefeed: error setting prompt")},
            }
            sh.try_wait_bg_jobs();
            match reader.read_line() {
                Ok(ReadResult::Input(mut line)) => {
                    line = input::remove_multiline_prompt(&line);
                    if line.trim() == "exit" {
                        println!("bye~");
                        return;
                    }
                    executor::run(&line, &mut sh);
                    // TODO: try wait 1000 times...
                    let mut i = 0;
                    loop {
                        sh.try_wait_bg_jobs();
                        i += 1;
                        if i == 1000 {
                            break;
                        }
                    }

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
