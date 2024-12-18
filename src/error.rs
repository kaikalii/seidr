use std::{error::Error, fmt, io};

use colored::{Color, Colorize};

use crate::{
    lex::{Ident, Role, Span},
    op::Op,
    value::Val,
};

#[derive(Debug, PartialEq, Eq)]
pub enum CompileError {
    IO(IoError),
    InvalidCharacter(char),
    InvalidNumber(String),
    InvalidEscape(String),
    Expected(String),
    ExpectedFound(String, String),
    UnclosedString,
    UnclosedChar,
    NoBinaryImplementation(Op),
    NoUnaryImplementation(Op),
    UnknownBinding(Ident),
    MismatchedRoles(Ident, Role),
    InvalidRole(Role, Vec<Role>),
    ParameterOutsideFunction,
    EmptyFunction,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CompileWarning {}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompileError::IO(e) => write!(f, "{}: {}", e.message, e.error),
            CompileError::InvalidCharacter(c) => write!(f, "Invalid character {:?}", c),
            CompileError::InvalidNumber(s) => write!(f, "Invalid number `{}`", s),
            CompileError::InvalidEscape(s) => write!(f, "Invalid escape `{}`", s),
            CompileError::UnclosedString => write!(f, "Unclosed string literal"),
            CompileError::UnclosedChar => write!(f, "Unclosed character literal"),
            CompileError::Expected(expected) => write!(f, "Expected {}", expected),
            CompileError::ExpectedFound(expected, found) => {
                write!(f, "Expected {}, found {}", expected, found)
            }
            CompileError::NoBinaryImplementation(op) => {
                write!(f, "{} has no binary implementation", op)
            }
            CompileError::NoUnaryImplementation(op) => {
                write!(f, "{} has no unary implementation", op)
            }
            CompileError::UnknownBinding(name) => write!(f, "Unknown binding `{}`", name),
            CompileError::MismatchedRoles(name, role) => {
                writeln!(
                    f,
                    "Mismatched roles\nThe name `{}` indicates a {}, but the body resolves to a {}.",
                    name,
                    name.role(),
                    role
                )?;
                match role {
                    Role::Value => write!(f, "Value names should start with a lowercase letter."),
                    Role::Function => {
                        write!(f, "Function names should start with an uppercase letter.")
                    }
                    Role::UnModifier => {
                        write!(f, "Unary modifier names should start with an underscore")
                    }
                    Role::BinModifier => write!(
                        f,
                        "Binary modifier names should start and end with an underscore"
                    ),
                }
            }
            CompileError::InvalidRole(found, expected) => {
                write!(f, "{} role is not valid in this position. Expected ", found)?;
                natural_list(expected, "or", f)
            }
            CompileError::ParameterOutsideFunction => {
                write!(f, "Parameters can only occur within functions")
            }
            CompileError::EmptyFunction => {
                write!(f, "Functions must contain at least one expression")
            }
        }
    }
}

fn natural_list<T>(items: &[T], conj: &str, f: &mut fmt::Formatter) -> fmt::Result
where
    T: fmt::Display,
{
    match items {
        [] => Ok(()),
        [item] => item.fmt(f),
        [a, b] => write!(f, "{} {} {}", a, conj, b),
        [initial @ .., last] => {
            for item in initial {
                write!(f, "{}, ", item)?;
            }
            write!(f, "{} {}", conj, last)
        }
    }
}

#[derive(Debug)]
pub struct IoError {
    pub message: String,
    pub error: io::Error,
}

impl fmt::Display for CompileWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {}
    }
}

impl CompileWarning {
    pub fn at(self, span: Span) -> Problem {
        Problem::Warning(SpannedCompileWarning { kind: self, span })
    }
}

impl CompileError {
    pub fn at(self, span: Span) -> Problem {
        Problem::Error(SpannedCompileError { kind: self, span })
    }
}

#[derive(Debug)]
pub struct SpannedCompileError {
    pub kind: CompileError,
    pub span: Span,
}

#[derive(Debug)]
pub struct SpannedCompileWarning {
    pub kind: CompileWarning,
    pub span: Span,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Problem {
    Error(SpannedCompileError),
    Warning(SpannedCompileWarning),
}

impl PartialEq for SpannedCompileError {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

impl PartialEq for SpannedCompileWarning {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

impl PartialEq for IoError {
    fn eq(&self, other: &Self) -> bool {
        self.message == other.message
    }
}

impl Eq for IoError {}
impl Eq for SpannedCompileError {}
impl Eq for SpannedCompileWarning {}

impl fmt::Display for SpannedCompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format_message("Error", Color::BrightRed, &self.kind.to_string(), f)?;
        self.span.format_error(f, Color::BrightRed)
    }
}

impl fmt::Display for SpannedCompileWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format_message("Warning", Color::BrightYellow, &self.kind.to_string(), f)?;
        self.span.format_error(f, Color::BrightYellow)
    }
}

fn format_message(
    error_kind: &str,
    error_color: Color,
    message: &str,
    f: &mut fmt::Formatter,
) -> fmt::Result {
    let mut lines = message.split('\n').map(str::trim);
    let padding = error_kind.chars().count() + 2;
    if let Some(line) = lines.next() {
        write!(
            f,
            "{} ",
            format!("{}:", error_kind).color(error_color).bold()
        )?;
        write!(f, "{}", line)?;
    }
    for line in lines {
        write!(f, "\n{:>padding$}{}", "", line, padding = padding)?;
    }
    Ok(())
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Problem::Error(e) => e.fmt(f),
            Problem::Warning(w) => w.fmt(f),
        }
    }
}

impl Error for Problem {}

pub type CompileResult<T = ()> = Result<T, Problem>;
pub type WarnedCompileResult<T> = Result<(T, Vec<SpannedCompileWarning>), Problem>;

impl Problem {
    pub fn prevents_compilation(&self) -> bool {
        match self {
            Problem::Error(_) => true,
            Problem::Warning(_) => false,
        }
    }
}

#[derive(Debug)]
pub struct RuntimeError {
    pub message: String,
    pub span: Option<Span>,
    pub trace: Vec<Span>,
}

impl From<fmt::Error> for RuntimeError {
    fn from(_: fmt::Error) -> Self {
        RuntimeError::new("formatting error", Span::dud())
    }
}

impl From<RuntimeError> for fmt::Error {
    fn from(_: RuntimeError) -> Self {
        fmt::Error
    }
}

impl RuntimeError {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        RuntimeError {
            message: message.into(),
            span: Some(span),
            trace: Vec::new(),
        }
    }
    pub fn trace_span(mut self, span: &Span) -> Self {
        self.trace.push(span.clone());
        self
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}",
            "Error: ".bright_red().bold(),
            self.message.bright_white()
        )?;
        if let Some(span) = &self.span {
            span.format_error(f, Color::BrightRed)?;
        }
        if !self.trace.is_empty() {
            writeln!(f)?;
            for span in &self.trace {
                span.format_error(f, Color::BrightRed)?;
            }
        }
        Ok(())
    }
}

pub type RuntimeResult<T = Val> = Result<T, RuntimeError>;
