use crate::codegen::Compiler;
use crate::lexer::Lexer;
use crate::parser::Parser;

pub struct UnoCompiler {}

impl UnoCompiler {
    pub fn compile(input: &str) {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();
        let parser_results = Parser::new(tokens).parse();

        Compiler::compile(parser_results.unwrap());
    }
}
