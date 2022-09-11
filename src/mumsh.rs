use std::collections::HashMap;

use nix::unistd::{tcsetpgrp, Pid};
use nix::sys::wait::{WaitPidFlag, WaitStatus, waitpid};

use crate::types::JobStatus;
use crate::{common, types::{self, Job}};

#[derive(Debug)]
pub struct Mumsh {
    pub fg_pgid: i32,
    pub current_dir: String,
    pub prev_dir: String,
    pub jobs: HashMap<i32, types::Job>  // key: pgid | value: Job
}

impl Mumsh {
    pub fn new() -> Self {
        Mumsh {
            fg_pgid: 0,
            current_dir: common::get_current_dir(),
            prev_dir: String::new(),
            jobs: HashMap::new(),
        }
    }

    pub fn set_foreground_pg(self: &mut Self, pgid: i32) -> bool {
        match tcsetpgrp(1, Pid::from_raw(pgid)) {
            Ok(_) => {
                self.fg_pgid = pgid;
                return true
            },
            Err(e) => {
                eprintln!("tcsetpgrp {}", e.to_string());
                return false;
            }
        }
    }

    pub fn insert_job(self: &mut Self, pgid: i32, pid: i32) {
        // pgid exists, push pid...
        if let Some(x) = self.jobs.get_mut(&pgid) {
            x.pids.push(pid);
            x.status.insert(pid, JobStatus::Running);
            return;
        }
        // find the smallest unused job_id...
        let mut job_id_vec = Vec::new();
        for (_, job) in self.jobs.iter() {
            job_id_vec.push(&job.id);
        }
        job_id_vec.sort();
        let mut job_id_new = 1;
        for (idx, job_id) in job_id_vec.iter().enumerate() {
            job_id_new = -1;
            if idx as i32 != **job_id - 1 {
                job_id_new = idx as i32 + 1;
                break;
            }
        }
        if job_id_new == -1 {
            job_id_new = self.jobs.len() as i32 + 1;
        }
        // insert job...
        self.jobs.insert(pgid, Job { 
            id: job_id_new, 
            pgid: pgid, 
            pids: vec![pid],
            status: HashMap::from([(pid, JobStatus::Running)]) 
        });
    }

    pub fn print_job(self: &Self, pgid: i32) {
        if let Some(x) = self.jobs.get(&pgid) {
            let mut string = format!("[{}]", x.id);
            for pid in &x.pids {
                string.push_str(format!(" {}", pid).as_str());
            }
            println!("{}", string);
        }
    }

    pub fn try_wait_bg_jobs(self: &mut Self) {
        let options = Some(WaitPidFlag::WUNTRACED |
                                                WaitPidFlag::WCONTINUED |  
                                                WaitPidFlag::WNOHANG);
        let mut finished_jobs = Vec::new();
        for (pgid, job) in self.jobs.iter_mut() {
            let mut i: i32 = 0;
            loop {
                let pid;
                match job.pids.iter().nth(i as usize) {
                    Some(x) => pid = x,
                    None => break,
                };
                match waitpid(Pid::from_raw(*pid), options) {
                    Ok(WaitStatus::Exited(pid, status)) => {
                        // println!("pid: {}, status: {}", pid, status);
                        job.status.insert(pid.as_raw(), JobStatus::Exited(status));
                        job.pids.remove(i as usize);
                        i -= 1;
                        if job.pids.is_empty() {
                            finished_jobs.push(*pgid);
                        }
                    },
                    Ok(WaitStatus::Signaled(pid, signal, bool)) => {
                        println!("{:?} {:?} {:?}", pid, signal, bool);  // TODO
                    },
                    Ok(WaitStatus::StillAlive) => {},
                    Ok(WaitStatus::Stopped(pid, signal)) => {
                        if signal.as_str() == "SIGTTIN" {
                            println!("suspended (tty input)");  // TODO format output?
                        }
                        job.status.insert(pid.as_raw(), JobStatus::Stopped);
                    },
                    Ok(x) => {println!("{:#?}", x)},    // TODO other situations?
                    Err(_) => {eprintln!("ERROR WAITPID!")},
                }
                i += 1;
            }
        }
        for pgid in finished_jobs {
            if let Some(job) = self.jobs.get(&pgid) {
                job.print_status();
            }
            self.jobs.remove(&pgid);
        }
    }
}
