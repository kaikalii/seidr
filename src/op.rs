use std::fmt;

macro_rules! op {
    ($name:ident, $(($variant:ident, $glyph:literal)),* $(,)?) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub enum $name {
            $($variant),*
        }

        impl $name {
            pub const fn glyph(&self) -> char {
                match self {
                    $($name::$variant => $glyph,)*
                }
            }
            pub const fn from_glyph(glyph: char) -> Option<Self> {
                match glyph {
                    $($glyph => Some($name::$variant),)*
                    _ => None,
                }
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{}", self)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                match self {
                    $($name::$variant => $glyph.fmt(f),)*
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Op {
    Pervasive(Pervasive),
    Rune(RuneOp),
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Pervasive {
    Math(MathOp),
    Comparison(ComparisonOp),
}

op!(
    MathOp,
    (Add, '+'),
    (Sub, '-'),
    (Mul, '×'),
    (Div, '÷'),
    (Max, '⎡'),
    (Min, '⎣'),
    (Mod, 'ᛁ')
);

op!(
    ComparisonOp,
    (Equal, '='),
    (NotEqual, '≠'),
    (Less, '<'),
    (LessOrEqual, '≤'),
    (Greater, '>'),
    (GreaterOrEqual, '≥'),
);

op!(
    RuneOp,
    (Fehu, 'ᚠ'),
    (Uruz, 'ᚢ'),
    (Ansuz, 'ᚨ'),
    (Kaunan, 'ᚲ'),
    (Gebo, 'ᚷ'),
    (Naudiz, 'ᚾ'),
    (Jera, 'ᛃ'),
    (Iwaz, 'ᛇ'),
    (Perth, 'ᛈ'),
    (Algiz, 'ᛉ'),
    (Sowilo, 'ᛊ'),
    (Tiwaz, 'ᛏ'),
    (Laguz, 'ᛚ'),
);

impl<P> From<P> for Op
where
    P: Into<Pervasive>,
{
    fn from(p: P) -> Self {
        Op::Pervasive(p.into())
    }
}

impl From<RuneOp> for Op {
    fn from(r: RuneOp) -> Self {
        Op::Rune(r)
    }
}

impl From<MathOp> for Pervasive {
    fn from(m: MathOp) -> Self {
        Pervasive::Math(m)
    }
}

impl From<ComparisonOp> for Pervasive {
    fn from(c: ComparisonOp) -> Self {
        Pervasive::Comparison(c)
    }
}

impl Op {
    pub const fn glyph(&self) -> char {
        match self {
            Op::Pervasive(p) => p.glyph(),
            Op::Rune(r) => r.glyph(),
        }
    }
    pub const fn from_glyph(glyph: char) -> Option<Self> {
        if let Some(p) = Pervasive::from_glyph(glyph) {
            Some(Op::Pervasive(p))
        } else if let Some(r) = RuneOp::from_glyph(glyph) {
            Some(Op::Rune(r))
        } else {
            None
        }
    }
}

impl Pervasive {
    pub const fn glyph(&self) -> char {
        match self {
            Pervasive::Math(m) => m.glyph(),
            Pervasive::Comparison(c) => c.glyph(),
        }
    }
    pub const fn from_glyph(glyph: char) -> Option<Self> {
        if let Some(m) = MathOp::from_glyph(glyph) {
            Some(Pervasive::Math(m))
        } else if let Some(c) = ComparisonOp::from_glyph(glyph) {
            Some(Pervasive::Comparison(c))
        } else {
            None
        }
    }
}

impl fmt::Debug for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Op::Pervasive(p) => p.fmt(f),
            Op::Rune(r) => r.fmt(f),
        }
    }
}

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Op::Pervasive(p) => p.fmt(f),
            Op::Rune(r) => r.fmt(f),
        }
    }
}

impl fmt::Debug for Pervasive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Pervasive::Math(m) => m.fmt(f),
            Pervasive::Comparison(c) => c.fmt(f),
        }
    }
}

impl fmt::Display for Pervasive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Pervasive::Math(m) => m.fmt(f),
            Pervasive::Comparison(c) => c.fmt(f),
        }
    }
}

op!(
    RuneUnMod,
    (Thurisaz, 'ᚦ'),
    (Raido, 'ᚱ'),
    (Wunjo, 'ᚹ'),
    (Berkanan, 'ᛒ'),
    (Ingwaz, 'ᛜ'),
    (Othala, 'ᛟ'),
);

op!(
    RuneBinMod,
    (Haglaz, 'ᚻ'),
    (Ehwaz, 'ᛖ'),
    (Mannaz, 'ᛗ'),
    (Dagaz, 'ᛞ'),
    (Stan, 'ᛥ'),
);
