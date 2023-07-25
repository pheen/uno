use crate::lexer::Token;

pub struct Call {
    pub fn_name: String,
    pub args: Vec<Node>,
}

pub struct InterpolableString {
    pub value: String,
}

pub enum Node {
    Call(Call),
    InterpolableString(InterpolableString),
}

pub struct ParserResult {
    pub ast: Vec<Node>,
}

pub struct Parser {
    pub tokens: Vec<Token>,
    pub pos: usize,
}

impl<'a> Parser {
    pub fn new(tokens: Vec<Token>) -> Parser {
        Parser { tokens, pos: 0 }
    }

    pub fn parse(&mut self) -> Result<ParserResult, &'static str> {
        let mut body = vec![];

        loop {
            if self.at_end() {
                break;
            }
            body.push(self.parse_unary_expr()?);
        }

        Ok(ParserResult { ast: body })
    }

    fn parse_unary_expr(&mut self) -> Result<Node, &'static str> {
        let result = self.parse_primary();

        result
    }

    fn parse_primary(&mut self) -> Result<Node, &'static str> {
        match self.current_token() {
            Token::Ident(_) => self.parse_ident_expr(),
            Token::StringLiteral(_) => self.parse_string_expr(),
            _ => Err("Unknown expression."),
        }
    }

    fn parse_ident_expr(&mut self) -> Result<Node, &'static str> {
        let ident = match self.current_token() {
            Token::Ident(name) => {
                self.advance();
                name
            }
            _ => return Err("Expected identifier."),
        };

        match self.current_token() {
            Token::LParen => {
                self.advance()?;

                if let Token::RParen = self.current_token() {
                    self.advance();

                    return Ok(Node::Call(Call {
                        fn_name: ident,
                        args: vec![],
                    }));
                }

                let mut args = vec![];

                loop {
                    args.push(self.parse_unary_expr()?);

                    match self.current_token() {
                        Token::RParen => {
                            self.advance();
                            break;
                        }
                        Token::Comma => {
                            self.advance();
                        }
                        _ => return Err("Expected ',' or ')' character in function call."),
                    }
                }

                Ok(Node::Call(Call {
                    fn_name: ident,
                    args,
                }))
            }

            _ => return Err("Expected a function call"),
        }
    }

    fn parse_string_expr(&mut self) -> Result<Node, &'static str> {
        match self.current_token() {
            Token::StringLiteral(string) => {
                self.advance();

                Ok(Node::InterpolableString(InterpolableString {
                    value: string,
                }))
            }
            _ => Err("Expected string literal."),
        }
    }

    fn current_token(&self) -> Token {
        self.tokens[self.pos].clone()
    }

    fn advance(&mut self) -> Result<(), &'static str> {
        let npos = self.pos + 1;

        self.pos = npos;

        if npos < self.tokens.len() {
            Ok(())
        } else {
            Err("Unexpected end of file.")
        }
    }

    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }
}
