use std::{
    borrow::Borrow,
    cmp::Ordering,
    fmt,
    fs::OpenOptions,
    io::Write,
    mem::take,
    ops::{Deref, DerefMut},
    path::{Path, MAIN_SEPARATOR},
    rc::Rc,
};

use colored::{Color, Colorize};

use crate::{error::*, num::Num, op::*};

pub fn lex<P>(input: &str, file: P) -> CompileResult<Vec<Token>>
where
    P: AsRef<Path>,
{
    // Get tokens
    let mut input = input.to_string();
    let tokens = loop {
        let mut lexer = Lexer::new(&input, &file);
        let tokens = lexer.lex()?;
        if lexer.escaped {
            // Write back to file
            match OpenOptions::new().write(true).truncate(true).open(&file) {
                Ok(mut file) => {
                    input = tokens.iter().map(|token| token.tt.to_string()).collect();
                    let _ = write!(file, "{}", input);
                }
                Err(error) => {
                    return Err(CompileError::IO(IoError {
                        message: format!("Unable to format `{}`", file.as_ref().to_string_lossy()),
                        error,
                    })
                    .at(Span::dud()))
                }
            }
        } else {
            break tokens;
        }
    };
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
pub struct Comment {
    pub message: Rc<str>,
    pub multiline: bool,
}

const SINGLE_LINE_COMMENT_CHAR: char = '᛫';
const MULTI_LINE_COMMENT_OPEN: char = '⌜';
const MULTI_LINE_COMMENT_CLOSE: char = '⌟';

impl fmt::Display for Comment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.multiline {
            write!(
                f,
                "{}{}{}",
                MULTI_LINE_COMMENT_OPEN, self.message, MULTI_LINE_COMMENT_CLOSE
            )
        } else {
            write!(f, "{} {}", SINGLE_LINE_COMMENT_CHAR, self.message)
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum TT {
    // Literals
    Num(Num, Rc<str>),
    Ident(Ident),
    Char(char),
    String(Rc<str>),
    Comment(Comment),
    // Ops
    Op(Op),
    UnMod(RuneUnMod),
    BinMod(RuneBinMod),
    // Brackets
    OpenParen,
    CloseParen,
    OpenCurly,
    CloseCurly,
    OpenAngle,
    CloseAngle,
    // Misc
    Comma,
    Whitespace,
    Newline,
    Undertie,
    SuperscriptMinus,
}

impl<O> From<O> for TT
where
    O: Into<Op>,
{
    fn from(op: O) -> Self {
        TT::Op(op.into())
    }
}

impl From<RuneUnMod> for TT {
    fn from(m: RuneUnMod) -> Self {
        TT::UnMod(m)
    }
}

impl From<RuneBinMod> for TT {
    fn from(m: RuneBinMod) -> Self {
        TT::BinMod(m)
    }
}

impl fmt::Debug for TT {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TT::Newline => write!(f, "\\n"),
            tt => write!(f, "{}", tt),
        }
    }
}

impl fmt::Display for TT {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TT::Num(n, s) => n.string_format(s).fmt(f),
            TT::Ident(ident) => ident.fmt(f),
            TT::Char(c) => write!(f, "{:?}", c),
            TT::String(s) => write!(f, "{:?}", s),
            TT::OpenParen => '('.fmt(f),
            TT::CloseParen => ')'.fmt(f),
            TT::OpenCurly => '{'.fmt(f),
            TT::CloseCurly => '}'.fmt(f),
            TT::OpenAngle => '⟨'.fmt(f),
            TT::CloseAngle => '⟩'.fmt(f),
            TT::Op(op) => op.fmt(f),
            TT::UnMod(m) => m.fmt(f),
            TT::BinMod(m) => m.fmt(f),
            TT::Comma => ','.fmt(f),
            TT::Newline => '\n'.fmt(f),
            TT::Undertie => '‿'.fmt(f),
            TT::SuperscriptMinus => '‾'.fmt(f),
            TT::Comment(comment) => comment.fmt(f),
            TT::Whitespace => ' '.fmt(f),
        }
    }
}

impl TT {
    pub fn is_sep(&self) -> bool {
        matches!(self, TT::Comma | TT::Newline)
    }
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
    pub fn sp<T>(self, data: T) -> Sp<T> {
        Sp { data, span: self }
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
            line_str.chars().take(self.loc.col - 1).collect::<String>(),
            line_str
                .chars()
                .skip(self.loc.col - 1)
                .take(self.len)
                .collect::<String>()
                .bright_white()
                .bold(),
            line_str
                .chars()
                .skip(self.loc.col - 1 + self.len)
                .collect::<String>()
        )?;
        write!(
            f,
            "{}{}",
            " ".repeat(self.loc.col + line_num.chars().count() + 2),
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
    escaped: bool,
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
            escaped: false,
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
    fn error<T>(&self, kind: CompileError) -> CompileResult<T> {
        Err(kind.at(self.span()))
    }
    fn token(&mut self, tt: impl Into<TT>) {
        if self.comment_depth > 0 {
            return;
        }
        self.tokens.push(Token {
            tt: tt.into(),
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
                '⟨' | '〈' | '[' => self.token(TT::OpenAngle),
                '⟩' | '〉' | ']' => self.token(TT::CloseAngle),
                ',' => self.token(TT::Comma),
                '‿' => self.token(TT::Undertie),
                '\n' => self.token(TT::Newline),
                '∞' => self.token(TT::Num(Num::INFINIFY, "∞".into())),
                '「' => self.token(MathOp::Max),
                '|' => self.token(MathOp::Mod),
                '"' => self.string()?,
                '\'' => {
                    if let Some(c) = self.char_literal('\'', CompileError::UnclosedChar)? {
                        if self.next() != Some('\'') {
                            return self.error(CompileError::UnclosedChar);
                        }
                        self.token(TT::Char(c));
                    } else {
                        return self.error(CompileError::UnclosedChar);
                    }
                }
                '\\' => self.escape()?,
                '‾' => self.negative_number()?,
                MULTI_LINE_COMMENT_OPEN => self.comment(MULTI_LINE_COMMENT_CLOSE, true),
                SINGLE_LINE_COMMENT_CHAR => self.comment('\n', false),
                c if digit_or_inf(c) => self.number(c, false)?,
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
                    while self.next_if(|c| c.is_whitespace() && c == '\n').is_some() {}
                    self.token(TT::Whitespace);
                }
                c => {
                    if let Some(op) = Op::from_glyph(c) {
                        self.token(op)
                    } else if let Some(m) = RuneUnMod::from_glyph(c) {
                        self.token(m)
                    } else if let Some(m) = RuneBinMod::from_glyph(c) {
                        self.token(m)
                    } else {
                        return self.error(CompileError::InvalidCharacter(c));
                    }
                }
            }
            self.start = self.loc;
        }
        Ok(take(&mut self.tokens))
    }
    fn comment(&mut self, terminator: char, multiline: bool) {
        let mut message = String::new();
        while let Some(c) = self.next_if(|c| c != terminator) {
            message.push(c);
        }
        self.next();
        self.token(TT::Comment(Comment {
            message: message.trim().into(),
            multiline,
        }))
    }
    fn escape(&mut self) -> CompileResult {
        let c = if let Some(c) = self.next() {
            c
        } else {
            return self.error(CompileError::InvalidEscape(String::new()));
        };
        match c {
            ' ' => self.token(TT::Undertie),
            '8' => self.token(TT::Num(Num::INFINIFY, "∞".into())),
            '-' => self.negative_number()?,
            '\\' => self.comment('\n', false),
            '*' => self.comment('*', true),
            c => {
                if let Some(op) = Op::from_escape(c) {
                    self.token(op);
                } else {
                    return self.error(CompileError::InvalidEscape(c.into()));
                }
            }
        };
        self.escaped = true;
        Ok(())
    }
    fn negative_number(&mut self) -> CompileResult {
        if let Some(c) = self.next_if(digit_or_inf) {
            self.number(c, true)
        } else {
            self.error(CompileError::Expected("digit".into()))
        }
    }
    fn number(&mut self, first: char, neg: bool) -> CompileResult {
        let neg = Num::from(if neg { -1i64 } else { 1 });
        if first == '∞' {
            self.token(TT::Num(Num::INFINIFY * neg, "‾∞".into()));
            return Ok(());
        }
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
            return self.error(CompileError::InvalidNumber(s));
        }
        if let Some(e) = self.next_if(|c| ['e', 'E'].contains(&c)) {
            s.push(e);
            if let Some(sign) = self.next_if(|c| ['+', '-', '‾'].contains(&c)) {
                s.push(sign);
            }
            while let Some(c) = self.next_if(ident_body_char) {
                s.push(c);
            }
            if !s.ends_with(|c: char| c.is_digit(10) || c == '_') {
                return self.error(CompileError::InvalidNumber(s));
            }
        }
        let normalized = s.replace('_', "").replace('‾', "-");
        match normalized.parse::<Num>() {
            Ok(num) => self.token(TT::Num(num * neg, s.into())),
            Err(_) => return self.error(CompileError::InvalidNumber(s)),
        }
        Ok(())
    }
    fn char_literal(
        &mut self,
        delimeter: char,
        error: CompileError,
    ) -> CompileResult<Option<char>> {
        let c = if let Some(c) = self.next() {
            c
        } else {
            return Ok(None);
        };
        Ok(Some(match c {
            '\\' => {
                if let Some(c) = self.next() {
                    match c {
                        '\\' => '\\',
                        '"' => '"',
                        'r' => '\r',
                        'n' => '\n',
                        't' => '\t',
                        '0' => '\0',
                        c if c == delimeter => delimeter,
                        c => c,
                    }
                } else {
                    return self.error(error);
                }
            }
            c if c == delimeter => return Ok(None),
            c => c,
        }))
    }
    fn string(&mut self) -> CompileResult {
        let mut s = String::new();
        while let Some(c) = self.char_literal('"', CompileError::UnclosedString)? {
            s.push(c);
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

pub fn ident_head_char(c: char) -> bool {
    !digit_or_inf(c) && ident_body_char(c)
}

pub fn ident_body_char(c: char) -> bool {
    c.is_alphanumeric() && !is_runic(c) || c == '_'
}

fn is_runic(c: char) -> bool {
    ('ᚠ'..='ᛪ').contains(&c)
}

pub fn digit_or_inf(c: char) -> bool {
    c.is_digit(10) || c == '∞'
}

#[derive(Clone)]
pub struct Sp<T> {
    pub data: T,
    pub span: Span,
}

impl<T> Sp<T> {
    pub fn map<F, U>(self, f: F) -> Sp<U>
    where
        F: FnOnce(T) -> U,
    {
        Sp {
            data: f(self.data),
            span: self.span,
        }
    }
}

impl<T> Sp<T>
where
    T: Clone,
{
    pub fn cloned(&self) -> T {
        self.data.clone()
    }
}

impl<T> Sp<T>
where
    T: Copy,
{
    pub fn copied(&self) -> T {
        self.data
    }
}

impl<T> fmt::Debug for Sp<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.data.fmt(f)
    }
}

impl<T> fmt::Display for Sp<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.data.fmt(f)
    }
}

impl<T> PartialEq for Sp<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl<T> Eq for Sp<T> where T: Eq {}

impl<T> PartialOrd for Sp<T>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.data.partial_cmp(&other.data)
    }
}

impl<T> Ord for Sp<T>
where
    T: Ord,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.data.cmp(&other.data)
    }
}

impl<T> AsRef<T> for Sp<T> {
    fn as_ref(&self) -> &T {
        &self.data
    }
}

impl<T> AsMut<T> for Sp<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.data
    }
}

impl<T> Deref for Sp<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> DerefMut for Sp<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}
