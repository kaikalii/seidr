#![allow(unused)]

mod ast;
mod checked;
mod error;
mod eval;
mod lex;
mod num;
mod op;
mod parse;
mod types;
mod value;

fn main() {
    let path = "main.sdr";
    let code = std::fs::read_to_string(path).unwrap();
    match parse::parse(&code, path) {
        Ok(items) => {
            let mut eval = eval::Evaler::default();
            for item in items {
                println!("\n    {}", item);
                match eval.item(item) {
                    Ok(()) => {}
                    Err(e) => println!("\n{}", e),
                }
            }
        }
        Err(e) => println!("{}", e),
    }
}
