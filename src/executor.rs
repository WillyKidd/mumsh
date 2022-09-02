use crate::parser::{self, parse_line::line_to_tokens};

pub fn run(line: &str) {
    println!("{:?}", parser::parse_line::split_line(line));
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
    let line_info = line_to_tokens(cmd);
    println!("{:?}", line_info.tokens);
    println!("{:?}", line_info.is_complete);
    println!("{:?}", line_info.unmatched);
    0
}