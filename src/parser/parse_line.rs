use crate::types::LineInfo;
use std::io::{self, Error, ErrorKind};

/// slit a line into multiple commandlines;
/// eg: sleep 10 && echo OK
///     -> ["sleep 10", "&&", "echo OK"]
pub fn split_line(line: &str) -> Vec<String> {
    let mut cmds = Vec::new();
    let mut token = String::new();
    let mut sep_stack = String::new();
    let mut token_len;
    let mut in_parenthesis = false;
    let mut _token;
    for c in line.chars() {
        if c == '\"' || c == '\'' || c == '`' {
            if in_parenthesis {
                match sep_stack.chars().last() {
                    Some(x) => {
                        if x == c {
                            sep_stack.pop();
                        } else {
                            sep_stack.push(c);
                            in_parenthesis = true;
                        }
                    },
                    None => {
                        sep_stack.push(c);
                        in_parenthesis = true;
                    }
                };
                if sep_stack.is_empty() {
                    in_parenthesis = false;
                }
            } else {
                sep_stack.push(c);
                in_parenthesis = true;
            }
        }
        if (c == '&' || c == '|') && !in_parenthesis {
            let token_last;
            match token.chars().last() {
                Some(x) => token_last = x,
                None => {
                    token.push(c);
                    continue;
                }
            };
            if token_last == c {
                token.push(c);
                token_len = token.len();
                if token_len - 2 > 0 {
                    _token = token[0..token_len-2].trim();
                    if !_token.is_empty() {
                        cmds.push(_token.to_string());
                    }
                }
                cmds.push(token[token_len-2..token_len].trim().to_string());
                token.clear();
                continue;
            }
        }
        if c == ';' && !in_parenthesis {
            _token = token.trim();
            if !_token.is_empty() {
                cmds.push(_token.to_string());
            }
            cmds.push(String::from(";"));
            token.clear();
        }
        token.push(c);
    }
    _token = token.trim();
    if !_token.is_empty() {
        cmds.push(_token.to_string());
    }
    cmds
}

/// check whether split result is valid
/// invalid: 
/// consecutive seperators -> Error::InvalidInput
///     eg: && &&
/// last seperator is a "&&" or "||" -> Error::Other
///     eg: ls &&
///     prompt for multiline input
pub fn check_split_result(tokens: &Vec<String>) -> io::Result<()> {
    let mut is_prev_sep = false;
    let len = tokens.len();
    for (i, token) in tokens.into_iter().enumerate() {
        let is_curr_sep = token == "&&" || token == "||" || token == ";";
        if i == 0 && (token == "&&" || token == "||") {
            return Err(Error::new(ErrorKind::InvalidInput, String::from(token)));
        } else if i == len - 1 && (token == "&&" || token == "||") {
            return Err(Error::new(ErrorKind::Other, String::from(token)));
        } else {
            if is_prev_sep && is_curr_sep {
                return Err(Error::new(ErrorKind::InvalidInput, String::from(token)));
            }
        }
        is_prev_sep = is_curr_sep;
    }
    Ok(())
}

pub fn line_to_tokens(line: &str) -> LineInfo {
    let mut quote_cnt = 0;
    let mut met_dollar = false;
    let mut met_parenthesis = false;
    let mut met_subshell;
    let mut token = String::new();
    let mut _token;
    let mut sep = String::new();
    let mut result = Vec::new();
    for (i, c) in line.chars().enumerate() {
        // mark met_dollar, indicating whether the last character is $ or not
        if c == '$' {
            token.push(c);
            continue;
        } else {
            if i > 1 {
                match line.chars().nth(i-1) {
                    Some(x) => {
                        if x == '$' {
                            met_dollar = true;
                        } else {
                            met_dollar = false;
                        }
                    },
                    None => {
                        met_dollar = false;
                    }
                };
            }
        }
        // quotes
        if c == '\'' || c == '\"' || c == '`' {
            let last_sep;
            match sep.chars().last() {
                Some(x) => last_sep = x,
                None => {
                    quote_cnt += 1;
                    sep.push(c);
                    continue;
                }
            };
            if quote_cnt > 0 {
                if last_sep == c {
                    sep.pop();
                    quote_cnt -= 1;
                    if token.is_empty() {                         // empty patenthesis, ignore
                        continue;
                    }
                    result.push((c.to_string(), token.clone()));  // do not trim
                    token.clear();
                    continue;
                } else {
                    token.push(c);
                    continue;
                }
            } else {
                quote_cnt += 1;
                sep.push(c);
                continue;
            }
        }
        // parenthesis begin
        if c == '(' || c == '{' {
            // check whether is $() or ${}
            if met_dollar {
                if met_parenthesis {                      // inside which parenthesis
                    match sep.chars().last() {
                        Some(y) => {
                            if y == '\"' {
                                met_subshell = true;
                            } else {
                                met_subshell = false;
                            }
                        },
                        None => {
                            met_subshell = true;
                        }
                    };
                } else {
                    met_subshell = true;
                }
            } else {
                met_subshell = false;
            }
            if met_subshell {
                sep.push(c);
                token.push(c);
                met_parenthesis = true;
                continue;
            } else {
                token.push(c);
                continue;
            }
        }
        // parenthesis end
        if c == ')' || c == '}' {
            token.push(c);
            let last_sep;
            match sep.chars().last() {
                Some(x) => last_sep = x,
                None => continue,
            };
            if (last_sep == '(' && c == ')') || (last_sep == '{' && c == '}') {
                sep.pop();
                met_parenthesis = false;
                
            }
            continue;
        }
        if c.is_whitespace() && quote_cnt == 0 && !met_parenthesis {
            _token = token.trim();
            if !_token.is_empty() {
                result.push((String::new(), _token.to_string()));
            }
            token.clear();
            continue;
        }
        token.push(c);
    }
    _token = token.trim();
    if !_token.is_empty() {
        result.push((String::new(), _token.to_string()));
    }
    LineInfo { tokens:result, is_complete:sep.is_empty(), unmatched:sep }
}
