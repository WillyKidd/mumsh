use crate::{types::CmdInfo, mumsh::Mumsh};

// returns 0 if all commands are builtin, returns -1 if need to run which
pub fn run(cmd_info: &mut CmdInfo, _sh: &mut Mumsh) -> i32 {
    let len = cmd_info.tokens.len();
    if len == 1 {
        return 0;
    }
    let mut to_delete = Vec::new();
    for (i, token) in cmd_info.tokens.iter_mut().skip(1).enumerate() {
        if token.1 == "cd" || token.1 == "which" {
            println!("{}: shell built-in command", token.1);
            to_delete.push(i+1);
        }
    }
    for (i, idx) in to_delete.iter().enumerate() {
        cmd_info.tokens.remove(*idx-i);
    }
    if to_delete.len() != len - 1 {
        return -1;
    } else {
        return 0;
    }
}
