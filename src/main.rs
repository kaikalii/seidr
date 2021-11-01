#![allow(unused)]

use crate::cwt::ToValNode;

mod array;
mod ast;
mod cwt;
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
                println!("    {}", expr);
                match expr.build_val_tree() {
                    Ok((val, warnings)) => {
                        for warning in warnings {
                            println!("{}", warning);
                        }
                        println!("{:?}", val);
                    }
                    Err(problems) => {
                        for problem in problems {
                            println!("{}", problem)
                        }
                    }
                }
            }
        }
        Err(e) => println!("{}", e),
    }
}
