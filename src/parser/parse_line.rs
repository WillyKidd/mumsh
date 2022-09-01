pub fn line_to_cmds(line: &str) -> Vec<String> {
    let mut cmds = Vec::new();
    let mut token = String::new();
    let mut token_len;
    let mut _token;
    for c in line.chars() {
        if c == '&' || c == '|' {
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
                    cmds.push(token[0..token_len-2].trim().to_string());
                }
                cmds.push(token[token_len-2..token_len].trim().to_string());
                token.clear();
                continue;
            }
        }
        if c == ';' {
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