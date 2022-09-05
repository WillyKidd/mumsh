use std::ffi::{CString, CStr};

use crate::parser;
use crate::types::{CmdlineInfo, CmdInfo};

use nix::unistd::{pipe, fork, execvp, close, getpid, setpgid, ForkResult, Pid, dup2};
use nix::sys::wait::wait;

/// run an entire line
pub fn run(line: &str) {
    // println!("{:?}", parser::parse_line::split_line(line));
    let mut status = 0;
    for token in parser::parse_line::split_line(line) {
        if token == "&&" && status != 0 {
            break;
        }
        if token == "||" && status == 0 {
            break;
        }
        if token == "||" || token == "&&" || token == ";" {
            continue;
        }
        status = run_cmdline(&token);
    }
}

/// run a sigle commandline that contains pipes
pub fn run_cmdline(cmd: &str) -> i32 {
    // let cmdline_info = CmdlineInfo::from(cmd);
    // let line_info = line_to_tokens(cmd);
    let cmdline_info = match CmdlineInfo::from(cmd) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("mumsh: {}", e);
            return -1;      // TODO: what to return?
        }
    };
    // println!("{:#?}", cmdline_info);
    let cmd_num = cmdline_info.cmds.len();

    // println!("{:?}", parser::parse_line::break_line_by_pipe(&line_info.tokens));
    // println!("{:?}", line_info.tokens);
    // println!("{:?}", line_info.is_complete);
    // println!("{:?}", line_info.unmatched);

    // parent: create all pipes and store in vec_pipes: pipe[0] read, pipe[1] write
    let mut vec_pipes = Vec::new();
    let mut pgid = 0;
    for _ in 0..cmd_num-1 {
        match pipe() {
            Ok(x) => vec_pipes.push(x),
            Err(e) => {
                eprintln!("mumsh: pipe error {}", e);
                return -1;
            }
        };
    }
    for (i, cmd) in cmdline_info.cmds.iter().enumerate() {
        run_single_cmd(cmd, cmd_num, i, &vec_pipes, &mut pgid);
    }
    // remember to close all unused pipes, otherwise EOF might be missed!
    for pipe in &vec_pipes {
        close(pipe.1).expect("Error closing pipe 1");
    }
    for _ in 0..cmd_num {
        wait().unwrap();    // TODO: background?
    }
    for pipe in &vec_pipes {
        close(pipe.0).expect("Error closing pipe 0");
    }
    0
}

/// run a single command, without pipes, but with redirections
pub fn run_single_cmd(cmd_info: &CmdInfo, cmd_num: usize, cmd_idx: usize, pipes: &Vec<(i32, i32)>, pgid: &mut i32) -> i32 {
    // fork
    match unsafe{fork()} {
        Ok(ForkResult::Parent { child, .. }) => {
            return child.as_raw();
        }
        Ok(ForkResult::Child) => {
            // Unsafe to use `println!` (or `unwrap`) here. See Safety.
            // setup pgid
            if cmd_idx == 0 {
                setpgid(Pid::from_raw(0), getpid()).expect("Error setting pgid");       // setup new process group
                *pgid = getpid().as_raw();
            } else {
                setpgid(Pid::from_raw(0), Pid::from_raw(*pgid)).expect("Error setting pgid");   // join process
            }
            // setup file descriptors
            for (i, pipe) in pipes.iter().enumerate() {     // close other pipes
                if cmd_idx > 0 {
                    if i != cmd_idx-1 {
                        close(pipe.0).expect("Error closing pipe 0");
                    }
                } else {
                    close(pipe.0).expect("Error closing pipe 0");
                }
                if i != cmd_idx {
                    close(pipe.1).expect("Error closing pipe 1");
                }
            }
            if cmd_idx > 0 {    // setup read end of pipe
                dup2(pipes[cmd_idx-1].0, 0).expect("Error duplicating file descriptor");
                close(pipes[cmd_idx-1].0).expect("Error closing pipe 0");
            }
            if cmd_idx < cmd_num - 1 {      // setup write end of pipe
                dup2(pipes[cmd_idx].1, 1).expect("Error duplicating file descriptor");
                close(pipes[cmd_idx].1).expect("Error closing pipe 0");
            }
            // setup execve arguments
            let c_file = CString::new(cmd_info.tokens[0].1.as_str()).expect("Error creating CString");
            let c_arg: Vec<CString> = cmd_info.tokens
                                            .iter()
                                            .map(|x| CString::new(x.1.as_str()).expect("Error creating CStr"))
                                            .collect();
            let c_arg_str: Vec<&CStr> = c_arg.iter().map(|x| x.as_c_str()).collect();
            match execvp(&c_file, &c_arg_str) {
                Ok(_) => {},
                Err(e) => {eprintln!("{}", e)}
            };
            unsafe { libc::_exit(0) };
        }
        Err(_) => {
            println!("Fork failed");
            return -1;
        }
    }
}
