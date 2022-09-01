use crate::parser;

pub fn run(line: &str) {
    println!("{:?}", parser::parse_line::line_to_cmds(line));
    let mut status = 0;
    for token in parser::parse_line::line_to_cmds(line) {
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
    println!("running command {}", cmd);
    0
}