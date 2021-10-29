#![allow(unused)]

mod array;
mod ast;
mod error;
mod eval;
mod lex;
mod num;
mod op;
mod parse;
mod value;

fn main() {
    let path = "main.sdr";
    let code = std::fs::read_to_string(path).unwrap();
    match parse::parse(&code, path) {
        Ok(exprs) => {
            for expr in exprs {
                println!("{}", expr);
            }
        }
        Err(e) => println!("{}", e),
    }
}
