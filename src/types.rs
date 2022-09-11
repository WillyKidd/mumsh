use std::collections::HashMap;

use crate::parser::parse_line;

pub type Token = (String, String);
pub type Tokens = Vec<Token>;

#[derive(Debug, Clone)]
pub struct RedirTo {
    pub redir_type: String,
    pub fd_before: i32,
    pub fd_after: i32,
    pub file_after: String
}

#[derive(Debug, Clone)]
pub struct RedirFrom {
    pub redir_type: String,
    pub fd_after: i32
}

#[derive(Debug, Clone)]
pub struct LineInfo {
    pub tokens: Tokens,
    pub is_complete: bool,
    pub here_doc: String,
    pub unmatched: String
}

#[derive(Debug, Clone)]
pub struct CmdInfo {
    pub tokens: Tokens,
    pub redir_from: Option<RedirFrom>,
    pub redir_to: Option<Vec<RedirTo>>
}

#[derive(Debug, Clone)]
pub struct CmdlineInfo {
    pub line: String,
    pub cmds: Vec<CmdInfo>,
    pub is_background: bool
}

impl CmdInfo {
    pub fn from(tokens: Tokens) -> Result<CmdInfo, String> {
        let cmd_info;
        match parse_line::tokens_check_redir_to(&tokens) {
            Ok(x) => cmd_info = x,
            Err(e) => return Err(e)
        };

        Ok(cmd_info)
    }
}

impl CmdlineInfo {
    pub fn from(line: &str) -> Result<CmdlineInfo, String> {
        let mut is_background = false;
        let mut cmds = Vec::new();
        let mut lineinfo = parse_line::line_to_tokens(line);
        // TODO: expand $(), ${}, ``...
        // let mut cmds = Vector::new();
        // check whether is background
        match lineinfo.tokens.iter().last() {
            Some(x) => {
                if x.1 == "&" {
                    is_background = true;
                }
            },
            None => {}
        };
        if is_background {
            lineinfo.tokens.pop();
        }
        // split tokens into vector of subtokens, seperated by pipes
        let sub_tokens: Vec<Tokens> = parse_line::break_line_by_pipe(&lineinfo.tokens);
        for sub_token in sub_tokens {
            let cmd_info = match CmdInfo::from(sub_token) {
                Ok(x) => x,
                Err(e) => return Err(e)
            };
            cmds.push(cmd_info);
        }
        Ok(CmdlineInfo { line: String::from(line), cmds: cmds, is_background: is_background })
    }
}


#[derive(Debug)]
pub struct Job {
    pub id: i32,                        // job id
    pub pgid: i32,                      // process group id
    pub pids: Vec<i32>,                 // pids that belong to this process group, that are still running
    pub status: HashMap<i32, JobStatus> // key: pid, value: job status
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum JobStatus {
    Running,
    Exited(i32),
    Stopped,
    None
}

impl Job {
    pub fn print_status(self: &Self) {
        if self.status.is_empty() {
            return;
        }
        let mut job_status_prev = JobStatus::None;
        let mut string = String::new();
        for (idx, (pid, job_status)) in self.status.iter().enumerate() {
            if *job_status == job_status_prev {
                string.push_str(format!("  {}", pid).as_str());
            } else {
                if !string.is_empty() {
                    println!("{}", string);
                    string = String::new();
                }
                if idx == 0 {
                    string.push_str(format!("{: <6}", format!("[{}]", self.id)).as_str());
                } else {
                    string.push_str("      ");
                }
                match job_status {
                    JobStatus::Running => string.push_str(format!("running <15").as_str()),
                    JobStatus::Exited(x) => {
                        if *x == 0 {
                            string.push_str(format!("{: <15}", "done").as_str());
                        } else {
                            string.push_str(format!("{: <15}", format!("exit {}", x)).as_str());
                        }
                    },
                    JobStatus::Stopped => string.push_str(format!("suspended <15").as_str()),
                    JobStatus::None => {},  // impossible...
                }
                job_status_prev = *job_status;
            }
        }
        if !string.is_empty() {
            println!("{}", string);
        }
    }
}
