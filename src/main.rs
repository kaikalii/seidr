#![allow(unused)]

mod error;
mod lex;
mod num;

fn main() {
    for c in ['ᚱ'] {
        println!("{}", c.is_alphabetic());
    }
    let path = "main.sdr";
    let code = std::fs::read_to_string(path).unwrap();
    match lex::lex_format(&code, path) {
        Ok(tokens) => {
            for token in tokens {
                println!("{:?}", token);
            }
        }
        Err(e) => println!("{}", e),
    }
}
