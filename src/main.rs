#![allow(unused)]

mod ast;
mod error;
mod eval;
mod lex;
mod num;
mod op;
mod parse;

fn main() {
    let path = "main.sdr";
    let code = std::fs::read_to_string(path).unwrap();
    match lex::lex(&code, path) {
        Ok(tokens) => {
            for token in tokens {
                println!("{:?}", token);
            }
        }
        Err(e) => println!("{}", e),
    }
}
