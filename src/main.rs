#![allow(unused)]

mod ast;
mod error;
mod ev;
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
        Ok(exprs) => {
            let mut eval = eval::Evaler::default();
            for expr in exprs {
                println!("\n    {}", expr);
                match eval.op_tree_expr(expr) {
                    Ok(ev) => println!("{}", ev),
                    Err(e) => println!("\n{}", e),
                }
            }
        }
        Err(e) => println!("{}", e),
    }
}
