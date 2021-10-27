use std::{
    borrow::Borrow,
    fmt,
    fs::{File, OpenOptions},
    io::Write,
    mem::take,
    ops::Deref,
    path::{Path, MAIN_SEPARATOR},
    rc::Rc,
};

use colored::{Color, Colorize};

use crate::{error::*, num::Num, op::Op};

pub fn lex<P>(input: &str, file: P) -> CompileResult<Vec<Token>>
where
    P: AsRef<Path>,
{
    let mut lexer = Lexer::new(input, &file);
    let tokens = lexer.lex()?;
    match OpenOptions::new().write(true).truncate(true).open(&file) {
        Ok(mut file) => {
            for token in &tokens {
                write!(file, "{}", token.tt).unwrap();
            }
        }
        Err(error) => {
            return lexer.error(CompileErrorKind::IO(IoError {
                message: format!("Unable to format `{}`", file.as_ref().to_string_lossy()),
                error,
            }))
        }
    }
    Ok(tokens)
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ident(Rc<str>);

impl From<String> for Ident {
    fn from(s: String) -> Self {
        Ident(s.into())
    }
}

impl From<Ident> for String {
    fn from(s: Ident) -> Self {
        (&*s.0).to_string()
    }
}

impl<'a> From<&'a str> for Ident {
    fn from(s: &'a str) -> Self {
        Ident(s.into())
    }
}

impl fmt::Debug for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for Ident {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for Ident {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl Deref for Ident {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq<str> for Ident {
    fn eq(&self, other: &str) -> bool {
        (&**self) == other
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TT {
    // Literals
    Num(Num, Rc<str>),
    Ident(Ident),
    String(Rc<str>),
    // Ops
    Op(Op),
    // Brackets
    OpenParen,
    CloseParen,
    OpenCurly,
    CloseCurly,
    OpenSquare,
    CloseSquare,
    // Misc
    Comma,
    Whitespace,
    Sep(String),
}

impl fmt::Display for TT {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TT::Num(n, s) => {
                if s.contains('e') || s.contains('E') {
                    s.fmt(f)
                } else {
                    let n = n.to_string();
                    let mut parts = n.split('.');
                    let left = parts.next().unwrap();
                    let right = parts.next();
                    for (i, c) in left.chars().enumerate() {
                        let i = left.len() - i - 1;
                        write!(f, "{}", c)?;
                        if i > 0 && i % 3 == 0 {
                            write!(f, "_")?;
                        }
                    }
                    if let Some(right) = right {
                        write!(f, ".")?;
                        for (i, c) in right.chars().enumerate() {
                            write!(f, "{}", c)?;
                            if i > 0 && i % 3 == 2 {
                                write!(f, "_")?;
                            }
                        }
                    }
                    Ok(())
                }
            }
            TT::Ident(ident) => ident.fmt(f),
            TT::String(s) => write!(f, "{:?}", s),
            TT::OpenParen => '('.fmt(f),
            TT::CloseParen => ')'.fmt(f),
            TT::OpenCurly => '{'.fmt(f),
            TT::CloseCurly => '}'.fmt(f),
            TT::OpenSquare => '['.fmt(f),
            TT::CloseSquare => ']'.fmt(f),
            TT::Op(op) => op.glyph().fmt(f),
            TT::Comma => ','.fmt(f),
            TT::Whitespace => ' '.fmt(f),
            TT::Sep(s) => s.fmt(f),
        }
    }
}

impl TT {
    pub fn keyword(ident: &str) -> Option<TT> {
        None
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Loc {
    pub pos: usize,
    pub line: usize,
    pub col: usize,
}

impl Loc {
    pub const fn start() -> Self {
        Loc {
            pos: 0,
            line: 1,
            col: 1,
        }
    }
}

#[derive(Clone)]
pub struct Span {
    pub loc: Loc,
    pub len: usize,
    pub input: Rc<[char]>,
    pub file: Rc<Path>,
}

impl Span {
    pub fn dud() -> Self {
        Span {
            loc: Loc::start(),
            len: 0,
            input: Rc::new([]),
            file: Rc::from("".as_ref()),
        }
    }
    pub fn as_string(&self) -> String {
        self.as_ref().iter().copied().collect()
    }
    pub fn line_string(&self) -> String {
        self.input
            .split(|&c| c == '\n')
            .nth(self.loc.line - 1)
            .unwrap()
            .iter()
            .collect()
    }
    pub fn join(&self, other: &Span) -> Span {
        let (start, end) = if self.loc.pos < other.loc.pos {
            (self, other)
        } else {
            (other, self)
        };
        Span {
            loc: start.loc,
            len: end.loc.pos + end.len - start.loc.pos,
            input: self.input.clone(),
            file: self.file.clone(),
        }
    }
    pub fn address(&self) -> String {
        let mut s = String::new();
        if !self.file.as_os_str().is_empty() {
            if self.file.is_relative() {
                s.push('.');
                s.push(MAIN_SEPARATOR);
            }
            s.push_str(&self.file.to_string_lossy());
            s.push(':');
        }
        s.push_str(&format!("{}:{}", self.loc.line, self.loc.col));
        s
    }
    pub fn format_error(&self, f: &mut fmt::Formatter, underline_color: Color) -> fmt::Result {
        write!(f, "{}", "\n --> ".bright_cyan())?;
        writeln!(f, "{}", self.address().bright_cyan())?;
        let line_num = self.loc.line.to_string();
        let line_str = self.line_string();
        writeln!(
            f,
            "{} | {}{}{}",
            line_num,
            &line_str[..self.loc.col - 1],
            line_str[self.loc.col - 1..self.loc.col - 1 + self.len]
                .bright_white()
                .bold(),
            &line_str[self.loc.col - 1 + self.len..]
        )?;
        write!(
            f,
            "{}{}",
            " ".repeat(self.loc.col + line_num.len() + 2),
            "^".repeat(self.len).color(underline_color).bold()
        )
    }
}

impl AsRef<[char]> for Span {
    fn as_ref(&self) -> &[char] {
        &self.input[self.loc.pos..self.loc.pos + self.len]
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.loc.line, self.loc.col)
    }
}

#[derive(Clone)]
pub struct Token {
    pub tt: TT,
    pub span: Span,
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} {:?}", self.tt, self.span)
    }
}

struct Lexer {
    input: Rc<[char]>,
    start: Loc,
    loc: Loc,
    file: Rc<Path>,
    tokens: Vec<Token>,
    comment_depth: usize,
}

impl Lexer {
    fn new<P>(input: &str, file: P) -> Self
    where
        P: AsRef<Path>,
    {
        Lexer {
            input: input.chars().collect::<Vec<_>>().into(),
            start: Loc::start(),
            loc: Loc::start(),
            file: file.as_ref().into(),
            tokens: Vec::new(),
            comment_depth: 0,
        }
    }
    fn peek(&mut self) -> Option<char> {
        self.input.get(self.loc.pos).copied()
    }
    fn next(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.loc.pos += 1;
        match ch {
            '\n' => {
                self.loc.line += 1;
                self.loc.col = 1;
            }
            '\r' => {}
            _ => self.loc.col += 1,
        }
        Some(ch)
    }
    fn next_if<F>(&mut self, f: F) -> Option<char>
    where
        F: FnOnce(char) -> bool,
    {
        if self.peek().filter(|&c| f(c)).is_some() {
            self.next()
        } else {
            None
        }
    }
    fn span(&self) -> Span {
        Span {
            loc: self.start,
            len: self.loc.pos - self.start.pos,
            input: self.input.clone(),
            file: self.file.clone(),
        }
    }
    fn error<T>(&self, kind: CompileErrorKind) -> CompileResult<T> {
        Err(kind.at(self.span()))
    }
    fn token(&mut self, tt: TT) {
        if self.comment_depth > 0 {
            return;
        }
        self.tokens.push(Token {
            tt,
            span: self.span(),
        });
    }
    fn token2(&mut self, ch: char, a: TT, b: TT) {
        if self.next_if(|c| c == ch).is_some() {
            self.token(b)
        } else {
            self.token(a)
        }
    }
    fn lex(&mut self) -> CompileResult<Vec<Token>> {
        while let Some(c) = self.next() {
            match c {
                '(' => self.token(TT::OpenParen),
                ')' => self.token(TT::CloseParen),
                '{' => self.token(TT::OpenCurly),
                '}' => self.token(TT::CloseCurly),
                '[' => self.token(TT::OpenSquare),
                ']' => self.token(TT::CloseSquare),
                ',' => self.token(TT::Comma),
                '"' => self.string()?,
                '\\' => self.escape()?,
                c if is_sep(c) => self.sep(c),
                c if c.is_digit(10) => self.number(c)?,
                c if ident_head_char(c) => {
                    let mut ident = String::from(c);
                    while let Some(c) = self.next_if(ident_body_char) {
                        ident.push(c);
                    }
                    self.token(
                        TT::keyword(&ident).unwrap_or_else(|| TT::Ident(ident.as_str().into())),
                    );
                }
                c if c.is_whitespace() => {
                    while self.next_if(char::is_whitespace).is_some() {}
                    self.token(TT::Whitespace);
                }
                c => {
                    if let Some(op) = Op::from_glyph(c) {
                        self.token(TT::Op(op))
                    } else {
                        return self.error(CompileErrorKind::InvalidCharacter(c));
                    }
                }
            }
            self.start = self.loc;
        }
        Ok(take(&mut self.tokens))
    }
    fn sep(&mut self, first: char) {
        let mut s = String::from(first);
        while let Some(c) = self.next_if(is_sep) {
            s.push(c);
        }
        self.token(TT::Sep(s));
    }
    fn escape(&mut self) -> CompileResult {
        let c = if let Some(c) = self.next() {
            c
        } else {
            return self.error(CompileErrorKind::InvalidEscape(String::new()));
        };
        match c {
            '*' => self.token(TT::Op(Op::Mul)),
            '/' => self.token(TT::Op(Op::Div)),
            '<' => self.token(TT::Op(Op::LessOrEqual)),
            '>' => self.token(TT::Op(Op::GreaterOrEqual)),
            '=' => self.token(TT::Op(Op::NotEqual)),
            c => return self.error(CompileErrorKind::InvalidEscape(c.into())),
        }
        Ok(())
    }
    fn number(&mut self, first: char) -> CompileResult {
        let mut s = String::from(first);
        while let Some(c) = self.next_if(|c| c.is_digit(10) || c == '_') {
            s.push(c);
        }
        if self.next_if(|c| c == '.').is_some() {
            s.push('.');
            while let Some(c) = self.next_if(|c| c.is_digit(10) || c == '_') {
                s.push(c);
            }
        }
        if s.ends_with('.') {
            return self.error(CompileErrorKind::InvalidNumber(s));
        }
        if let Some(e) = self.next_if(|c| ['e', 'E'].contains(&c)) {
            s.push(e);
            if let Some(sign) = self.next_if(|c| ['+', '-'].contains(&c)) {
                s.push(sign);
            }
            while let Some(c) = self.next_if(ident_body_char) {
                s.push(c);
            }
            if !s.ends_with(|c: char| c.is_digit(10) || c == '_') {
                return self.error(CompileErrorKind::InvalidNumber(s));
            }
        }
        let no_underscores = s.replace('_', "");
        match no_underscores.parse::<Num>() {
            Ok(num) => self.token(TT::Num(num, s.into())),
            Err(_) => return self.error(CompileErrorKind::InvalidNumber(s)),
        }
        Ok(())
    }
    fn string(&mut self) -> CompileResult {
        let mut s = String::new();
        let mut escaped = false;
        let mut closed = false;
        while let Some(c) = self.next() {
            match c {
                '\\' if escaped.take() => s.push('\\'),
                '"' if escaped.take() => s.push('"'),
                'r' if escaped.take() => s.push('\r'),
                'n' if escaped.take() => s.push('\n'),
                't' if escaped.take() => s.push('\t'),
                '0' if escaped.take() => s.push('\0'),
                '\\' => escaped = true,
                '"' => {
                    closed = true;
                    break;
                }
                c => s.push(c),
            }
        }
        if !closed {
            return self.error(CompileErrorKind::UnclosedString);
        }
        self.token(TT::String(s.into()));
        Ok(())
    }
}

trait BoolTake {
    fn take(&mut self) -> bool;
}

impl BoolTake for bool {
    fn take(&mut self) -> bool {
        let res = *self;
        *self = false;
        res
    }
}

fn ident_head_char(c: char) -> bool {
    !c.is_digit(10) && ident_body_char(c)
}

fn ident_body_char(c: char) -> bool {
    c.is_alphanumeric() && !is_runic(c) || c == '_'
}

fn is_runic(c: char) -> bool {
    ('ᚠ'..='ᛪ').contains(&c)
}

fn is_sep(c: char) -> bool {
    ",\n".contains(c)
}
