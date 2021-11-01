#![allow(unused)]

use crate::{
    cwt::ToValNode,
    eval::{Eval, Runtime},
};

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
            let mut rt = Runtime::default();
            for expr in exprs {
                println!("    {}", expr);
                match expr.build_val_tree() {
                    Ok((node, warnings)) => {
                        for warning in warnings {
                            println!("{}", warning);
                        }
                        match node.eval(&mut rt) {
                            Ok(val) => println!("{}", val),
                            Err(e) => println!("\n{}", e),
                        }
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
