use std::ffi::{CString, CStr};

use crate::builtin;
use crate::{parser, mumsh::Mumsh};
use crate::types::{CmdlineInfo, CmdInfo};

use nix::sys::stat::Mode;
use nix::unistd::{dup2, pipe, fork, execvp, close, getpid, setpgid, ForkResult, Pid, getpgid};
use nix::sys::wait::wait;
use nix::fcntl::{open, OFlag};

pub fn try_run_builtin(cmd_info: &mut CmdInfo, sh: &mut Mumsh) -> Option<i32> {
    let token_first = match cmd_info.tokens.first() {
        Some(x) => x,
        None => return None
    };
    if token_first.1 == "cd" {
        return Some(builtin::cd::run(cmd_info, sh));
    }
    if token_first.1 == "which" {
        if builtin::which::run(cmd_info, sh) == 0 {
            return Some(0);
        }
    }
    None
}

/// run an entire line
pub fn run(line: &str, sh: &mut Mumsh) {
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
        status = run_cmdline(&token, sh);
    }
}

/// run a sigle commandline that contains pipes
pub fn run_cmdline(cmd: &str, sh: &mut Mumsh) -> i32 {
    let mut cmdline_info = match CmdlineInfo::from(cmd) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("mumsh: {}", e);
            return -1;      // TODO: what to return?
        }
    };
    let cmd_num = cmdline_info.cmds.len();
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
    let mut pid_first_child = 0;
    let mut cmds_to_wait = 0;
    for (i, cmd) in cmdline_info.cmds.iter_mut().enumerate() {
        let pid_child = run_single_cmd(cmd, cmd_num, i, &vec_pipes, sh, &mut pgid);
        if pid_first_child == 0 {
            pid_first_child = pid_child;
        }
        if pid_child > 0 {
            cmds_to_wait += 1;
        }
    }
    // donate tty to child
    if pid_first_child != 0 {
        Mumsh::set_foreground_pg(pid_first_child);
    }
    // remember to close all unused pipes, otherwise EOF might be missed!
    for pipe in &vec_pipes {
        close(pipe.1).expect("Error closing pipe 1");
    }
    for _ in 0..cmds_to_wait {
        wait().unwrap();    // TODO: background? unwrap panic?
    }
    // reclaim tty
    if pid_first_child != 0 {
        let pgid = getpgid(Some(Pid::from_raw(0))).expect("Error getting pgid").as_raw();
        Mumsh::set_foreground_pg(pgid);
    }
    for pipe in &vec_pipes {
        close(pipe.0).expect("Error closing pipe 0");
    }
    0
}

/// run a single command, without pipes, but with redirections
pub fn run_single_cmd(cmd_info: &mut CmdInfo, cmd_num: usize, cmd_idx: usize, pipes: &Vec<(i32, i32)>, sh: &mut Mumsh, pgid: &mut i32) -> i32 {
    // fork
    let dup_error = "mumsh: error duplicating file descriptor";
    let close_error = "mumsh: error closing file descriptor";
    let cstring_error = "mumsh: error creating cstring";
    // check if builtin, return 0 if builtin
    if let Some(_) = try_run_builtin(cmd_info, sh) {
        return 0;
    }
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
            match &cmd_info.redir_to {      // check redir_to
                Some(vec_redir_to) => {
                    for redir_to in vec_redir_to {
                        if redir_to.redir_type == ">&" {
                            dup2(redir_to.fd_after, redir_to.fd_before).expect(dup_error);
                        } else if redir_to.redir_type == ">" || redir_to.redir_type == ">>" {
                            // try to open file
                            let oflag;
                            let fd_after;
                            if redir_to.redir_type == ">" {
                                oflag = OFlag::O_WRONLY | OFlag::O_CREAT | OFlag::O_TRUNC;
                            } else {
                                oflag = OFlag::O_APPEND | OFlag::O_CREAT | OFlag::O_WRONLY;
                            }
                            match open(redir_to.file_after.as_str(), oflag, Mode::S_IRWXU) {
                                Ok(x) => fd_after = x,
                                Err(e) => {
                                    eprintln!("{}", e);
                                    unsafe { libc::_exit(0) };
                                }     // TODO: bash style error handling
                            };
                            // redirect
                            dup2(fd_after, redir_to.fd_before).expect(dup_error);
                            close(fd_after).expect(close_error);
                        }
                    }
                },
                None => {}
            };
            // TODO: redir_from
            for (i, pipe) in pipes.iter().enumerate() {     // close other pipes
                if cmd_idx > 0 {
                    if i != cmd_idx-1 {
                        close(pipe.0).expect(close_error);
                    }
                } else {
                    close(pipe.0).expect(close_error);
                }
                if i != cmd_idx {
                    close(pipe.1).expect(close_error);
                }
            }
            if cmd_idx > 0 {    // setup read end of pipe
                dup2(pipes[cmd_idx-1].0, 0).expect(dup_error);
                close(pipes[cmd_idx-1].0).expect(close_error);
            }
            if cmd_idx < cmd_num - 1 {      // setup write end of pipe
                dup2(pipes[cmd_idx].1, 1).expect(dup_error);
                close(pipes[cmd_idx].1).expect(close_error);
            }
            
            // setup execve arguments
            let c_file = CString::new(cmd_info.tokens[0].1.as_str()).expect(cstring_error);
            let c_arg: Vec<CString> = cmd_info.tokens
                                            .iter()
                                            .map(|x| CString::new(x.1.as_str()).expect(cstring_error))
                                            .collect();
            let c_arg_str: Vec<&CStr> = c_arg.iter().map(|x| x.as_c_str()).collect();
            match execvp(&c_file, &c_arg_str) {
                Ok(_) => {},
                Err(e) => {eprintln!("{}", e)}      // TODO: bash style error handling
            };
            unsafe { libc::_exit(0) };
        }
        Err(_) => {
            println!("Fork failed");
            return -1;
        }
    }
}
