#![allow(unused, clippy::match_single_binding)]
#![warn(unused_imports, unused_must_use, unreachable_patterns)]

use std::fs::read_to_string;

use cwt::TreeBuilder;
use runtime::Runtime;

use crate::{ast::Item, eval::Eval, format::Format};

mod array;
mod ast;
mod cwt;
mod error;
mod eval;
mod format;
mod function;
mod lex;
mod num;
mod op;
mod parse;
mod pervade;
mod rcview;
mod runtime;
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

    let mut builder = TreeBuilder::default();
    let rt = Runtime::default();
    let mut nodes = Vec::new();
    let mut errored = false;
    for item in items {
        match item {
            Item::Newline | Item::Comment(_) => {}
            Item::Expr(expr) => match builder.build(&expr) {
                Ok((node, warnings)) => {
                    nodes.push((expr, node));
                    for warning in warnings {
                        println!("{}", warning);
                    }
                }
                Err(problems) => {
                    errored = true;
                    for problem in problems {
                        println!("{}", problem)
                    }
                }
            },
        }
    }

    if errored {
        return;
    }

    for (expr, node) in nodes {
        println!();
        println!("    {}", expr.expr);
        match node.eval(&rt).and_then(|val| val.as_string()) {
            Ok(s) => println!("{}", s),
            Err(e) => {
                println!("\n{}", e);
                break;
            }
        }
    }
}
