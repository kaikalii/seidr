use std::fmt;

use crate::{
    array::Array,
    error::RuntimeResult,
    lex::{digit_or_inf, ident_body_char, ident_head_char},
    value::{Atom, Val},
};

pub trait Format {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()>;
    fn as_string(&self) -> RuntimeResult<String> {
        let mut string = String::new();
        let mut formatter = Formatter::new(&mut string);
        self.format(&mut formatter)?;
        Ok(string)
    }
}

pub struct Formatter<'w> {
    indent: usize,
    writer: &'w mut dyn fmt::Write,
    prev_alphanum: bool,
}

impl<'w> Formatter<'w> {
    pub fn new<W: fmt::Write>(writer: &'w mut W) -> Self {
        Formatter {
            indent: 0,
            writer,
            prev_alphanum: false,
        }
    }
    fn write_str(&mut self, s: &str) {
        if self.prev_alphanum && s.starts_with(|c| ident_head_char(c) || digit_or_inf(c)) {
            write!(self.writer, " ").unwrap_or_else(|e| panic!("{}", e));
        }

        self.prev_alphanum = s.ends_with(|c| ident_body_char(c) || digit_or_inf(c));

        write!(self.writer, "{}", s).unwrap_or_else(|e| panic!("{}", e));
    }
    pub fn display<T>(&mut self, val: T)
    where
        T: fmt::Display,
    {
        self.write_str(&val.to_string());
    }
    pub fn debug<T>(&mut self, val: T)
    where
        T: fmt::Debug,
    {
        self.write_str(&format!("{:?}", val));
    }
    pub fn indent(&mut self, delta: usize) {
        self.indent += delta;
    }
    pub fn deindent(&mut self, delta: usize) {
        self.indent -= delta;
    }
    pub fn newline(&mut self) {
        self.display('\n');
        for _ in 0..self.indent {
            self.display(' ');
        }
    }
    pub fn array(&mut self, array: &Array) -> RuntimeResult<()> {
        let unbounded = array.len().is_none();
        let array = array.bounded();
        match array.limited_depth()? {
            0 => unreachable!(),
            1 => {
                if array.len().expect("finite array has no length") > 0
                    && array
                        .iter()
                        .all(|val| matches!(val.as_deref(), Ok(Val::Atom(Atom::Char(_)))))
                {
                    let mut s = String::new();
                    for val in array.iter() {
                        if let Val::Atom(Atom::Char(c)) = val?.as_ref() {
                            s.push(*c);
                        }
                    }
                    let s = format!("{:?}", s);
                    self.display(&s[..s.len() - 1]);
                    if unbounded {
                        self.display("...");
                    }
                    self.display("\"");
                } else {
                    self.display("⟨");
                    for (i, val) in array.iter().enumerate() {
                        let val = val?;
                        if i > 0 {
                            self.display(" ");
                        }
                        val.format(self)?;
                    }
                    if unbounded {
                        self.display(" ...");
                    }
                    self.display("⟩");
                }
            }
            depth => {
                for item in array.iter() {
                    self.newline();
                    match item?.into_owned() {
                        Val::Atom(atom) => atom.format(self)?,
                        Val::Array(arr) => arr.format(self)?,
                    }
                }
            }
        }
        Ok(())
    }
}
