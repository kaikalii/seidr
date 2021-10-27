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
    match parse::parse(&code, path) {
        Ok(items) => {
            for item in items {
                println!("{:#?}", item);
            }
        }
        Err(e) => println!("{}", e),
    }
}
