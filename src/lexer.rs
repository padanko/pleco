use std::collections::VecDeque;

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Command(char),
    MultiLengthCommand(String),
    Argument(String),
    Var(String),
    Integer(i32),
    Code(String),
}

pub struct Lexer {
    pub input: VecDeque<char>,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
        }
    }

    pub fn next_token(&mut self) -> Option<Token> {
        while let Some(c) = self.input.pop_front() {
            match c {
                '*' => {
                    let mut arg = String::new();
                    while let Some(nc) = self.input.pop_front() {
                        if nc == ';' {
                            return Some(Token::Integer(arg.parse().unwrap_or(0)));
                        } else {
                            arg.push(nc);
                        }
                    }
                    return Some(Token::Integer(arg.parse().unwrap_or(0)));
                }
                '{' => {
                    let mut arg = String::new();
                    let mut escape_mode = false;
                    while let Some(nc) = self.input.pop_front() {
                        if nc == '\\' && !escape_mode {
                            escape_mode = true;
                        }
                        if nc == '\\' && escape_mode {
                            arg.push('\\');
                            escape_mode = false;
                        }
                        if nc == 'n' && escape_mode {
                            arg.push('\n');
                            escape_mode = false;
                        }
                        if nc == '}' && !escape_mode {
                            return Some(Token::Code(arg));
                        } else {
                            arg.push(nc);
                        }
                    }
                    return Some(Token::Code(arg));
                }
                '^' => {
                    let mut arg = String::new();
                    while let Some(nc) = self.input.pop_front() {
                        if nc == '^' {
                            return Some(Token::MultiLengthCommand(arg));
                        } else {
                            arg.push(nc);
                        }
                    }
                    return Some(Token::MultiLengthCommand(arg));
                }
                '"' => {
                    let mut escape_mode = false;
                    let mut arg = String::new();
                    while let Some(nc) = self.input.pop_front() {
                        if nc == '\\' && !escape_mode {
                            escape_mode = true;
                        } else if escape_mode && nc == '\\' {
                            arg.push('\\');
                            escape_mode = false;
                        } else if escape_mode && nc == '"' {
                            arg.push('"');
                            escape_mode = false;
                        } else if escape_mode && nc == 'n' {
                            arg.push('\n');
                            escape_mode = false;
                        } else if nc == '"' && !escape_mode {
                            return Some(Token::Argument(arg));
                        } else {
                            arg.push(nc);
                        }
                    }
                    return Some(Token::Argument(arg)); // 引用符が閉じられなかった場合
                }
                '$' => {
                    let mut arg = String::new();
                    while let Some(nc) = self.input.pop_front() {
                        if nc == '$' {
                            return Some(Token::Var(arg));
                        } else {
                            arg.push(nc);
                        }
                    }
                    return Some(Token::Var(arg));
                }
                _ => return Some(Token::Command(c)),
            }
        }
        None
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        while let Some(token) = self.next_token() {
            tokens.push(token);
        }
        tokens
    }
}

pub fn extract_argument(token: Token) -> Option<String> {
    match token {
        Token::Argument(arg) => Some(arg),
        _ => None,
    }
}

pub fn extract_var(token: Token) -> Option<String> {
    match token {
        Token::Var(arg) => Some(arg),
        _ => None,
    }
}

pub fn extract_integer(token: Token) -> Option<i32> {
    match token {
        Token::Integer(arg) => Some(arg),
        _ => None,
    }
}
