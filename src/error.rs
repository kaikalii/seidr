use std::{error::Error, fmt, io};

use colored::{Color, Colorize};

use crate::lex::Span;

#[derive(Debug, PartialEq, Eq)]
pub enum CompileErrorKind {
    IO(IoError),
    InvalidCharacter(char),
    InvalidNumber(String),
    InvalidEscape(String),
    ExpectedFound(String, String),
    UnclosedString,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CompileWarningKind {}

impl fmt::Display for CompileErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompileErrorKind::IO(e) => write!(f, "{}: {}", e.message, e.error),
            CompileErrorKind::InvalidCharacter(c) => write!(f, "Invalid character {:?}", c),
            CompileErrorKind::InvalidNumber(s) => write!(f, "Invalid number `{}`", s),
            CompileErrorKind::InvalidEscape(s) => write!(f, "Invalid escape `{}`", s),
            CompileErrorKind::UnclosedString => write!(f, "Unclosed string literal"),
            CompileErrorKind::ExpectedFound(expected, found) => {
                write!(f, "Expected {}, found {}", expected, found)
            }
        }
    }
}

#[derive(Debug)]
pub struct IoError {
    pub message: String,
    pub error: io::Error,
}

impl fmt::Display for CompileWarningKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {}
    }
}

impl CompileWarningKind {
    pub fn at(self, span: Span) -> Problem {
        Problem::Warning(CompileWarning { kind: self, span })
    }
}

impl CompileErrorKind {
    pub fn at(self, span: Span) -> Problem {
        Problem::Error(CompileError { kind: self, span })
    }
}

#[derive(Debug)]
pub struct CompileError {
    pub kind: CompileErrorKind,
    pub span: Span,
}

#[derive(Debug)]
pub struct CompileWarning {
    pub kind: CompileWarningKind,
    pub span: Span,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Problem {
    Error(CompileError),
    Warning(CompileWarning),
}

impl PartialEq for CompileError {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

impl PartialEq for CompileWarning {
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
impl Eq for CompileError {}
impl Eq for CompileWarning {}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", "Error: ".bright_red().bold())?;
        let message = self.kind.to_string();
        write!(f, "{}", message.bright_white())?;
        self.span.format_error(f, Color::BrightRed)
    }
}

impl fmt::Display for CompileWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", "Warning: ".bright_yellow().bold())?;
        let message = self.kind.to_string();
        write!(f, "{}", message.bright_white())?;
        self.span.format_error(f, Color::BrightYellow)
    }
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

pub type CompileResult<T> = Result<T, Problem>;
pub type WarnedCompileResult<T> = Result<(T, Vec<CompileWarning>), Problem>;

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
    pub trace: Vec<String>,
}

impl RuntimeError {
    pub fn new(message: impl Into<String>, span: Option<Span>) -> Self {
        RuntimeError {
            message: message.into(),
            span,
            trace: Vec::new(),
        }
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
            for item in &self.trace {
                write!(f, "\n{}", item)?;
            }
        }
        Ok(())
    }
}

pub type RuntimeResult<T = ()> = Result<T, RuntimeError>;
