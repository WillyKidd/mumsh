use linefeed::{Function, Prompter, Terminal};
use std::io::{self, ErrorKind};

use crate::parser;

pub struct InputCheck;

impl<Term: Terminal> Function<Term> for InputCheck {
    fn execute(&self, prompter: &mut Prompter<Term>, count: i32, _ch: char) -> io::Result<()> {
        let buf = prompter.buffer();
        let split_result = parser::parse_line::split_line(buf);
        let mut complete_prompt = String::new();
        match parser::parse_line::check_split_result(&split_result) {
            Ok(_) => {},
            Err(e) => {
                match e.kind() {
                    ErrorKind::InvalidInput => return Err(e),
                    ErrorKind::Other => {
                        match e.to_string().as_str() {
                            "&&" => complete_prompt = String::from("cmdand> "),
                            "||" => complete_prompt = String::from("cmdor> "),
                            _ => eprintln!("split_result: unknown error string")
                        };
                    },
                    _ => eprintln!("split_result: unknown error"),
                }
            }
        };

        let parse_result = parser::parse_line::line_to_tokens(buf);

        if !complete_prompt.is_empty() && count > 0 {       
            match prompter.insert(count as usize, '\n') {
                Ok(_) => {},
                Err(e) => eprintln!("input-check error: {}", e),
            };
            match prompter.insert_str(&complete_prompt) {
                Ok(_) => {},
                Err(e) => eprintln!("input-check error: {}", e),
            };
            return Ok(());
        }

        if parse_result.is_complete {
            return prompter.accept_input();
        } else if count > 0 {
            let mut char_mismatch = ' ';
            complete_prompt = String::new();
            // check heredoc, other wise incomplete quote or braces
            if !parse_result.here_doc.is_empty() {
                complete_prompt.push_str("heredoc> ");
            } else {
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
            }
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
        .replace("\ncmdand> ", "")
        .replace("\ncmdor> ", "")
        .replace("\nheredoc> ", "\n")     // trick: retains \n for heredoc
        .replace("\n> ", "")
        .to_string()
}
