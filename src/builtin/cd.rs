use std::{env, ffi::CString};
use std::fs::metadata;


use crate::{types::CmdInfo, mumsh::Mumsh};

use libc::{chdir, perror};
use home;

pub fn run(cmd_info: &mut CmdInfo, sh: &mut Mumsh) -> i32 {
    // chdir(path);
    let c_cd = CString::new("cd").unwrap();
    let argc = cmd_info.tokens.len();
    let home = home::home_dir().unwrap();
    let mut new_cwd = String::new();
    let mut print_new_pwd = false;
    // expand ~ into $HOME
    for token in cmd_info.tokens.iter_mut() {
        if let Some(x) = token.1.chars().nth(0) {
            if x == '~' && token.0 != "\'" {
                token.1 = home.as_path().display().to_string();
            }
        }
    }
    // cd old new
    if argc >= 3 {
        new_cwd = sh.current_dir.clone();
        new_cwd = new_cwd.replacen(&cmd_info.tokens[1].1, &cmd_info.tokens[2].1, 1);
        print_new_pwd = true;
    }
    if argc == 2 {
        if cmd_info.tokens[1].1 == "-" {
            new_cwd = sh.prev_dir.clone();
        } else {
            if let Some(x) = cmd_info.tokens[1].1.chars().nth(0) {
                if x == '/' {   // absolute path
                    new_cwd = cmd_info.tokens[1].1.clone();
                } else {        // relative path, first check current dir, then check CDPATH
                    new_cwd = cmd_info.tokens[1].1.clone();
                    let mut check_cdpath = false;
                    match metadata(&new_cwd) {
                        Ok(md) => {
                            if !md.is_dir() {
                                eprint!("cd: not a directory: {}", cmd_info.tokens[1].1);
                                return -1;
                            }
                        },
                        Err(_) => check_cdpath = true,
                    };
                    if check_cdpath {
                        if let Ok(x) = env::var("CDPATH") {
                            new_cwd = x.clone();
                            new_cwd.push_str(&cmd_info.tokens[1].1);
                            print_new_pwd = true;
                        }
                    }
                }
            }
        }
    }
    if argc == 1 {
        new_cwd = home.as_path().display().to_string();
    }
    if new_cwd.len() == 0 {
        eprint!("cd: no such file or directory: {}", cmd_info.tokens[1].1);
        return -1;
    }
    // call chdir after path finalized
    let c_path = CString::new(new_cwd).unwrap();
    unsafe {
        if chdir(c_path.as_ptr()) == 0 {
            let pwd_env = env::current_dir().unwrap();
            env::set_var("OLDPWD", &sh.current_dir);
            sh.prev_dir = sh.current_dir.clone();
            env::set_var("PWD", &pwd_env);
            sh.current_dir = pwd_env.into_os_string().into_string().unwrap();
            if print_new_pwd {
                let mut pwd_short = sh.current_dir.clone();
                pwd_short = pwd_short.replacen(&home.as_path().display().to_string() ,"~", 1);
                println!("{}", pwd_short);
            }
            return 0;
        } else {
            perror(c_cd.as_ptr());
            return -1;
        }
    }
}
