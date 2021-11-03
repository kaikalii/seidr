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

use notify::{event::ModifyKind, Event, EventKind, RecursiveMode, Watcher};

fn main() {
    ctrlc::set_handler(|| exit(0));

    // Init file watcher
    let (path_send, path_recv) = channel();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| match res {
        Ok(event) => {
            if let EventKind::Modify(_) = event.kind {
                if let Some(path) = event
                    .paths
                    .into_iter()
                    .find(|path| path.extension().map_or(false, |ext| ext == "sdr"))
                {
                    let _ = path_send.send(path);
                }
            }
        }
        Err(e) => println!("{}", e),
    })
    .unwrap();
    let watch_path = Path::new(".");

    let mut watch = |watch: bool| {
        if watch {
            watcher.watch(watch_path, RecursiveMode::Recursive).unwrap();
        } else {
            watcher.unwatch(watch_path).unwrap();
        }
    };
    watch(true);

    // Listen for changes
    for path in path_recv {
        watch(false);

        // Read in file
        let code = match read_to_string(&path) {
            Ok(code) => code,
            Err(e) => {
                println!("{}", e);
                watch(true);
                continue;
            }
        };

        // Parse file
        let items = match parse::parse(&code, path) {
            Ok(exprs) => exprs,
            Err(e) => {
                println!("{}", e);
                watch(true);
                continue;
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

        println!();
        watch(true);
    }
}
