use std::{error::Error, fmt, io};

use colored::{Color, Colorize};

use crate::{eval::Const, lex::Span, op::Op, types::Type};

#[derive(Debug, PartialEq, Eq)]
pub enum CompileError {
    IO(IoError),
    InvalidCharacter(char),
    InvalidNumber(String),
    InvalidEscape(String),
    ExpectedFound(String, String),
    UnclosedString,
    UnclosedChar,
    IncompatibleBinTypes(Op, Const, Const),
    IncompatibleUnType(Op, Const),
    NoBinaryImplementation(Op),
    NoUnaryImplementation(Op),
    DifferentArraySizes(Op, Const, Const),
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
            CompileError::ExpectedFound(expected, found) => {
                write!(f, "Expected {}, found {}", expected, found)
            }
            CompileError::IncompatibleBinTypes(op, left, right) => {
                write!(f, "{} {} {} is invalid", left.ty(), op, right.ty())
            }
            CompileError::IncompatibleUnType(op, inner) => {
                write!(f, "{} {} is invalid", op, inner.ty())
            }
            CompileError::NoBinaryImplementation(op) => {
                write!(f, "{} has no binary implementation", op)
            }
            CompileError::NoUnaryImplementation(op) => {
                write!(f, "{} has no unary implementation", op)
            }
            CompileError::DifferentArraySizes(op, left, right) => {
                write!(
                    f,
                    "Different-sized arrays {} and {} are not compatible with {}",
                    left.ty(),
                    right.ty(),
                    op
                )
            }
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
        write!(f, "{}", "Error: ".bright_red().bold())?;
        let message = self.kind.to_string();
        write!(f, "{}", message.bright_white())?;
        self.span.format_error(f, Color::BrightRed)
    }
}

impl fmt::Display for SpannedCompileWarning {
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
