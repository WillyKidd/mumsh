use crate::types::{LineInfo, Tokens, Token, CmdInfo, RedirTo};

use std::io::{self, Error, ErrorKind};
use regex::Regex;

/// slit a line into multiple commandlines;
/// control operators: & && || ;
/// eg: sleep 10 && echo OK
///     -> ["sleep 10", "&&", "echo OK"]
pub fn split_line(line: &str) -> Vec<String> {
    let mut cmds = Vec::new();
    let mut token = String::new();
    let mut sep_stack = String::new();
    let mut token_len;
    let mut in_quotes = false;
    let mut _token;
    for (i, c) in line.chars().enumerate() {
        // quotes
        if c == '\"' || c == '\'' || c == '`' {
            if in_quotes {
                match sep_stack.chars().last() {
                    Some(x) => {
                        if x == c {
                            sep_stack.pop();
                        } else {
                            sep_stack.push(c);
                            in_quotes = true;
                        }
                    },
                    None => {
                        sep_stack.push(c);
                        in_quotes = true;
                    }
                };
                if sep_stack.is_empty() {
                    in_quotes = false;
                }
            } else {
                sep_stack.push(c);
                in_quotes = true;
            }
        }
        // &
        if (c == '&') && !in_quotes {
            let mut background = true;
            // >& or &&, not background
            match line.chars().nth(i-1) {
                Some(x) => {
                    if x == '>' || x == '&' {
                        background = false;
                    }
                },
                None => {},
            };
            match line.chars().nth(i+1) {
                Some(x) => {
                    if x == '&' {
                        background = false;
                    }
                },
                None => {},
            }
            // otherwise, break the command
            if background {
                token.push(' ');    // trick: add a space
                token.push(c);
                cmds.push(token);
                token =  String::new();
                continue;
            }
        }
        // && ||
        if (c == '&' || c == '|') && !in_quotes {
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
        // ;
        if c == ';' && !in_quotes {
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
        } else if i == len - 1 {
            if token == "&&" || token == "||" {
                return Err(Error::new(ErrorKind::Other, String::from(token)));
            }
            if token.ends_with("<<") {
                return Err(Error::new(ErrorKind::InvalidInput, String::from("<<")));
            }
        } else {
            if is_prev_sep && is_curr_sep {
                return Err(Error::new(ErrorKind::InvalidInput, String::from(token)));
            }
        }
        is_prev_sep = is_curr_sep;
    }
    Ok(())
}

/// split a line into tokens
/// eg: echo "11\n22" | wc -l
///     => [("", "echo"), 
///         ("\"", "11\n22"),
///         ("", "|"),
///         ("", "wc"),
///         ("", "-l")]
pub fn line_to_tokens(line: &str) -> LineInfo {
    let mut quote_cnt = 0;
    let mut met_dollar = false;
    let mut met_parenthesis = false;
    let mut met_subshell;
    let mut token = String::new();
    let mut _token;
    let mut sep = String::new();
    let mut result = Vec::new();
    let mut is_complete;
    let mut heredoc_string = String::new();
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
        // pipe
        if c == '|' && !met_parenthesis && quote_cnt == 0 {
            _token = token.trim();
            if !_token.is_empty() {
                result.push((String::new(), _token.to_string()));
            }
            token.clear();
            result.push((String::new(), String::from('|')));
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
    // if line complete, check for heredoc <<
    is_complete = sep.is_empty();
    if is_complete {
        for (i, token) in result.iter().enumerate() {
            if token.1 == "<<" && token.0.is_empty() {
                let len = result.len();
                is_complete = false;
                match result.iter().nth(i+1) {
                    Some(x) => heredoc_string.push_str(&x.1),
                    None => is_complete = true
                };
                match result.iter().nth(len - 1) {
                    Some(x) => {
                        if heredoc_string == x.1 && i+1 != len-1 {
                            is_complete = true;
                            heredoc_string.clear();
                        }
                    },
                    None => {}
                };
            }
        }
    }
    LineInfo { tokens:result, is_complete:is_complete, here_doc:heredoc_string, unmatched:sep }
}

/// split tokens into many tokens by pipes
/// eg: echo "11\n22" | wc -l
///     [("", "echo"), ("\"", "11\n22"), ("", "|"), ("", "wc"), ("", "-l")]
///         => [[("", "echo"), ("\"", "11\n22")],
///             [("", "wc"), ("", "-l")]]
pub fn break_line_by_pipe(tokens: &Tokens) -> Vec<Tokens> {
    let mut result = Vec::new();
    let mut temp: Vec<Token> = Vec::new();
    for token in tokens.iter() {
        if token.1 == "|" {
            result.push(temp);  // move temp to result
            temp = Vec::new();  // construct new temp
        } else {
            temp.push((token.0.clone(), token.1.clone()));
        }
    }
    if !temp.is_empty() {
        result.push(temp);
    }
    return result;
}

/// checks each token and generate CmdInfo, which contains:
///     tokens: finalized tokens without redirection info
///     redir_to: a vector of redirect_to information
///         > >> >& supported
///     redir_from: TODO
pub fn tokens_check_redir_to(tokens: &Tokens) -> Result<CmdInfo, String> {
    let mut tokens_result = Vec::new();
    let mut redir_to_result = Vec::new();
    let re_redir_to_fd = Regex::new(r"(^[1-9]|^)>&([1-9]|$)").unwrap();
    let re_redir_append = Regex::new(r"(^[1-9]|^)?>>(.*)").unwrap();
    let re_redir = Regex::new(r"(^[1-9]|^)?>(.*)").unwrap();
    let mut skip_next = false;
    for (i, token) in tokens.iter().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }
        let mut is_redir_to_fd = false;
        let mut is_redir_to = false;
        let mut redir_to = RedirTo{ redir_type: String::new(), fd_before: -1, fd_after: -1 , file_after: String::new() };
        if token.0 == "\'" || token.0 == "\"" {
            continue;
        }
        // check if contains >&
        match re_redir_to_fd.captures(&token.1) {
            Some(x) => {
                is_redir_to_fd = true;
                redir_to.redir_type.push_str(">&");
                redir_to.fd_before = match x[1].parse() {
                    Ok(x) => x,
                    Err(_) => -1
                };
                redir_to.fd_after = match x[2].parse() {
                    Ok(x) => x,
                    Err(_) => -1
                };
            },
            None => {}
        };
        // check whether >& is complete
        if is_redir_to_fd {
            if redir_to.fd_before == -1 {                   // fd_before not found, check previous token
                if i > 0 {
                    match tokens.iter().nth(i-1) {
                        Some(x) => {
                            redir_to.fd_before = match x.1.parse() {
                                Ok(x) => {
                                    tokens_result.pop();
                                    x
                                },
                                Err(_) => 1                 // if previous token is not numeric, set to stdout
                            };
                        },
                        None => redir_to.fd_before = 1
                    };
                }
            }
            if redir_to.fd_after == -1 {                    // fd_after not found, check next token
                match tokens.iter().nth(i+1) {
                    Some(x) => {
                        redir_to.fd_after = match x.1.parse() {
                            Ok(x) => {
                                skip_next = true;
                                x
                            },
                            Err(_) => {
                                return Err(String::from(">&"));
                            }
                        };
                    }
                    None => {
                        return Err(String::from(">&"));
                    }
                };
            }
            // RedirTo constructed
            redir_to_result.push(redir_to);
            let token_remaining = String::from(re_redir_to_fd.replace(&token.1, ""));
            if !token_remaining.is_empty() {
                tokens_result.push((token.0.clone(), token_remaining));
            }
            continue;
        }
        // check whether contains >>
        match re_redir_append.captures(&token.1) {
            Some(x) => {
                is_redir_to = true;
                redir_to.redir_type.push_str(">>");
                redir_to.fd_before = match x[1].parse() {
                    Ok(x) => x,
                    Err(_) => -1
                };
                if !&x[2].is_empty() {
                    redir_to.file_after = String::from(&x[2]);
                }
            },
            None => {}
        };
        // check whether contains >
        if !is_redir_to {
            match re_redir.captures(&token.1) {
                Some(x) => {
                    is_redir_to = true;
                    redir_to.redir_type.push_str(">");
                    match x.get(1) {
                        Some(x) => match x.as_str().parse() {
                            Ok(y) => redir_to.fd_before = y,
                            Err(_) => redir_to.fd_before = -1,
                        },
                        None => redir_to.fd_before = -1
                    };
                    if !&x[2].is_empty() {
                        redir_to.file_after = String::from(&x[2]);
                    }
                },
                None => {}
            };
        }
        if is_redir_to {
            if redir_to.fd_before == -1 {
                redir_to.fd_before = 1;
                if i > 0 {
                    match tokens.iter().nth(i-1) {
                        Some(x) => {
                            redir_to.fd_before = match x.1.parse() {
                                Ok(x) => {
                                    tokens_result.pop();
                                    x
                                },
                                Err(_) => 1
                            };
                        },
                        None => redir_to.fd_before = 1
                    };
                }
            }
            if redir_to.file_after.is_empty() {
                match tokens.iter().nth(i+1) {
                    Some(x) => {
                        redir_to.file_after.push_str(&x.1);
                        skip_next = true;
                    }
                    None => {
                        return Err(String::from(">"));
                    }
                };
            }
            // RedirTo constructed
            let token_remaining;
            if redir_to.redir_type == ">>" {
                token_remaining = String::from(re_redir_append.replace(&token.1, ""));
            } else {
                token_remaining = String::from(re_redir.replace(&token.1, ""));
            }
            if !token_remaining.is_empty() {
                tokens_result.push((token.0.clone(), token_remaining));
            }
            redir_to_result.push(redir_to);
            continue;
        }
        tokens_result.push(token.clone());
    }
    let ret_redir_to;
    // let ret_redir_from;
    if redir_to_result.is_empty() {
        ret_redir_to = None;
    } else {
        ret_redir_to = Some(redir_to_result);
    }
    Ok( CmdInfo { tokens: tokens_result, redir_from: None, redir_to: ret_redir_to } )  // TODO
}
