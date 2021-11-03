#![allow(unused)]

use std::{fs::read_to_string, path::Path, process::exit, sync::mpsc::channel};

use crate::{
    ast::Item,
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

    // Read in file
    let code = match read_to_string(&path) {
        Ok(code) => code,
        Err(e) => {
            println!("{}", e);
            return;
        }
    };

    // Parse file
    let items = match parse::parse(&code, path) {
        Ok(exprs) => exprs,
        Err(e) => {
            println!("{}", e);
            return;
        }
    };

    let mut rt = Runtime::default();
    for item in items {
        match item {
            Item::Comment(_) => {}
            Item::Expr(expr) => {
                println!("    {}", expr.expr);
                match expr.expr.build_val_tree() {
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
    }
}
