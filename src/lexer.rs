use std::{iter::Peekable, str::Chars};

#[derive(Clone)]
pub enum Token {
    Comma,
    Ident(String),
    LParen,
    NewLine,
    RParen,
    Space,
    StringLiteral(String),
}

pub struct Lexer<'a> {
    input: &'a str,
    chars: Box<Peekable<Chars<'a>>>,
    char_pos: usize,
}

impl Lexer<'_> {
    pub fn new(input: &str) -> Lexer {
        Lexer {
            input,
            chars: Box::new(input.chars().peekable()),
            char_pos: 0,
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = vec![];

        while let Some(token) = self.lex() {
            match token {
                Token::Space => {}
                Token::NewLine => {}
                _ => tokens.push(token),
            }
        }

        tokens
    }

    pub fn lex(&mut self) -> Option<Token> {
        let ch = match self.chars.next() {
            Some(ch) => ch,
            None => return None,
        };

        let mut pos = self.char_pos;
        let start = pos;

        pos += 1;

        let token = match ch {
            '(' => Token::LParen,
            ')' => Token::RParen,
            ',' => Token::Comma,
            'a'..='z' | 'A'..='Z' | '_' => {
                loop {
                    let ch = match self.chars.peek() {
                        Some(ch) => *ch,
                        None => break,
                    };

                    if ch != '_' && !(ch).is_alphanumeric() {
                        break;
                    }

                    self.chars.next();

                    pos += 1;
                }

                let ident = self.input[start..pos].to_string();

                Token::Ident(ident)
            }
            '"' => {
                loop {
                    let ch = self.chars.next();

                    pos += 1;

                    let ch = match ch {
                        Some(ch) => ch,
                        None => break,
                    };

                    if let '"' = ch {
                        break;
                    }
                }

                Token::StringLiteral(self.input[start + 1..pos - 1].to_string())
            }
            ' ' => {
                self.chars.next();

                loop {
                    match self.chars.peek() {
                        Some(c) => match c {
                            ' ' => {
                                self.chars.next();
                            }
                            _ => break,
                        },
                        None => break,
                    }
                }

                Token::Space
            }
            '\n' => {
                self.chars.next();

                loop {
                    match self.chars.peek() {
                        Some(c) => match c {
                            '\n' => {
                                self.chars.next();
                            }
                            _ => break,
                        },
                        None => break,
                    }
                }

                Token::NewLine
            }
            _ => {
                todo!()
            }
        };

        self.char_pos = pos;

        Some(token)
    }
}
