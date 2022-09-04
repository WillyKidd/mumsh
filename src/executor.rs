use crate::parser;
use crate::types::CmdlineInfo;

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
        status = run_cmd(&token);
    }
}

pub fn run_cmd(cmd: &str) -> i32 {
    // let cmdline_info = CmdlineInfo::from(cmd);
    // let line_info = line_to_tokens(cmd);
    CmdlineInfo::from(cmd);
    // println!("{:?}", parser::parse_line::break_line_by_pipe(&line_info.tokens));
    // println!("{:?}", line_info.tokens);
    // println!("{:?}", line_info.is_complete);
    // println!("{:?}", line_info.unmatched);
    0
}