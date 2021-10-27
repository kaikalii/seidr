use std::{
    fmt,
    io::{self, Write},
};

use crate::{
    lex::{Ident, Span},
    num::Num,
    op::Op,
};

pub enum Item {
    Expr(Expr),
}

impl fmt::Debug for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Item::Expr(expr) => expr.fmt(f),
        }
    }
}

pub enum Expr {
    Num(Num, Span),
    Char(char, Span),
    Ident(Ident, Span),
    Un(Box<UnExpr>),
    Bin(Box<BinExpr>),
}

impl Expr {
    pub fn span(&self) -> &Span {
        match self {
            Expr::Char(_, span) | Expr::Num(_, span) | Expr::Ident(_, span) => span,
            Expr::Un(expr) => &expr.span,
            Expr::Bin(expr) => &expr.span,
        }
    }
}

impl fmt::Debug for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Num(n, _) => n.fmt(f),
            Expr::Char(c, _) => c.fmt(f),
            Expr::Ident(ident, _) => ident.fmt(f),
            Expr::Un(expr) => expr.fmt(f),
            Expr::Bin(expr) => expr.fmt(f),
        }
    }
}

pub struct UnExpr {
    pub op: Op,
    pub inner: Expr,
    pub op_span: Span,
    pub span: Span,
    pub parened: bool,
}

impl fmt::Debug for UnExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({} {:?})", self.op, self.inner)
    }
}

pub struct BinExpr {
    pub op: Op,
    pub left: Expr,
    pub right: Expr,
    pub op_span: Span,
    pub span: Span,
    pub parened: bool,
}

impl fmt::Debug for BinExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:?} {} {:?})", self.left, self.op, self.right)
    }
}

pub struct Formatter {
    depth: usize,
    writer: Box<dyn Write>,
}

impl Formatter {
    pub fn new<W: Write + 'static>(writer: W) -> Self {
        Formatter {
            depth: 0,
            writer: Box::new(writer),
        }
    }
}

impl Write for Formatter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.writer.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

pub trait Format {
    fn format(&self, f: &mut Formatter) -> io::Result<()>;
}

impl Format for Item {
    fn format(&self, f: &mut Formatter) -> io::Result<()> {
        match self {
            Item::Expr(expr) => expr.format(f),
        }
    }
}

impl Format for Expr {
    fn format(&self, f: &mut Formatter) -> io::Result<()> {
        match self {
            Expr::Num(n, s) => {
                let s = s.as_string();
                if s.contains('e') || s.contains('E') {
                    write!(f, "{}", s)
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
            Expr::Char(c, _) => write!(f, "{:?}", c),
            Expr::Ident(ident, _) => write!(f, "{}", ident),
            Expr::Un(expr) => expr.format(f),
            Expr::Bin(expr) => expr.format(f),
        }
    }
}

impl Format for BinExpr {
    fn format(&self, f: &mut Formatter) -> io::Result<()> {
        if self.parened {
            write!(f, "(")?;
        }
        self.left.format(f)?;
        write!(f, " {} ", self.op)?;
        self.right.format(f)?;
        if self.parened {
            write!(f, ")")?;
        }
        Ok(())
    }
}

impl Format for UnExpr {
    fn format(&self, f: &mut Formatter) -> io::Result<()> {
        if self.parened {
            write!(f, "(")?;
        }
        write!(f, "{} ", self.op)?;
        self.inner.format(f)?;
        if self.parened {
            write!(f, ")")?;
        }
        Ok(())
    }
}
