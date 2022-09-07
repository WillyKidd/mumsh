use nix::unistd::{tcsetpgrp, Pid};

#[derive(Debug)]
pub struct Mumsh {
    
}

impl Mumsh {
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
