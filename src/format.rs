use std::fmt;

use crate::{
    error::RuntimeResult,
    lex::{digit_or_inf, ident_body_char, ident_head_char},
};

pub struct Formatter<'w> {
    depth: usize,
    writer: &'w mut dyn fmt::Write,
    prev_alphanum: bool,
}

impl<'w> Formatter<'w> {
    pub fn new<W: fmt::Write>(writer: &'w mut W) -> Self {
        Formatter {
            depth: 0,
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
    pub fn newline(&mut self) {
        self.display('\n')
    }
}

pub trait Format {
    fn format(&self, f: &mut Formatter) -> RuntimeResult<()>;
    fn as_string(&self) -> RuntimeResult<String> {
        let mut string = String::new();
        let mut formatter = Formatter::new(&mut string);
        self.format(&mut formatter)?;
        Ok(string)
    }
}
