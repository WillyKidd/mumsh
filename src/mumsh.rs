use nix::unistd::{tcsetpgrp, Pid};

use crate::common;

#[derive(Debug)]
pub struct Mumsh {
    pub current_dir: String,
    pub prev_dir: String, 
}

impl Mumsh {
    pub fn new() -> Self {
        Mumsh {
            current_dir: common::get_current_dir(),
            prev_dir: String::new()
        }
    }

    pub fn set_foreground_pg(pgid: i32) -> bool {
        match tcsetpgrp(1, Pid::from_raw(pgid)) {
            Ok(_) => return true,
            Err(e) => {
                eprintln!("tcsetpgrp {}", e.to_string());
                return false;
            }
        }
    }
}
