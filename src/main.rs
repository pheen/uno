mod codegen;
mod lexer;
mod parser;
mod uno_compiler;

use uno_compiler::UnoCompiler;

pub fn main() {
    let input = std::fs::read_to_string("dev.uno").unwrap();
    UnoCompiler::compile(&input);
}
