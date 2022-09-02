use linefeed::{Function, Prompter, Terminal};
use std::io;

use crate::parser;

pub struct InputCheck;

impl<Term: Terminal> Function<Term> for InputCheck {
    fn execute(&self, prompter: &mut Prompter<Term>, count: i32, _ch: char) -> io::Result<()> {
        let buf = prompter.buffer();
        let parse_result = parser::parse_line::line_to_tokens(buf);
        if parse_result.is_complete {
            return prompter.accept_input();
        } else if count > 0 {
            let mut char_mismatch = ' ';
            let mut complete_prompt = String::new();
            match parse_result.unmatched.chars().last() {
                Some(x) => char_mismatch = x,
                None => {},
            };
            match char_mismatch {
                '\"' | '\'' | '`' => complete_prompt.push_str("dquote> "),
                '{' => complete_prompt.push_str("braceparam> "),
                '(' => complete_prompt.push_str("cmdsubst> "),
                _ => complete_prompt.push_str("> ")
            };
            match prompter.insert(count as usize, '\n') {
                Ok(_) => {},
                Err(e) => eprintln!("input-check error: {}", e),
            };
            match prompter.insert_str(&complete_prompt) {
                Ok(_) => {},
                Err(e) => eprintln!("input-check error: {}", e),
            };
        }
        Ok(())
    }
}

pub fn remove_multiline_prompt(line: &str) -> String {
    line.replace("\ndquote> ", "")
        .replace("\nbraceparam> ", "")
        .replace("\ncmdsubst> ", "")
        .replace("\n> ", "")
        .to_string()
}
