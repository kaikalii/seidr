use std::fmt;

macro_rules! op {
    ($name:ident, $($(#[$meta:meta])* ($variant:ident, $glyph:literal $(,$escape:literal)?)),* $(,$no_glyph:ident)* $(,)?) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub enum $name {
            $($(#[$meta])* $variant,)*
            $($no_glyph,)*
        }

        impl $name {
            pub const fn from_glyph(glyph: char) -> Option<Self> {
                match glyph {
                    $($glyph => Some($name::$variant),)*
                    _ => None,
                }
            }
            pub const fn from_escape(escape: char) -> Option<Self> {
                match escape {
                    $($($escape => Some($name::$variant),)*)*
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
                    $($name::$no_glyph => stringify!($no_glyph).fmt(f),)*
                }
            }
        }
    }
}

op!(AssignOp, (Assign, '←', '\''), (Reassign, '↩', '"'));

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Op {
    Pervasive(Pervasive),
    Rune(RuneOp),
    Other(OtherOp),
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
    (Mul, '×', 'x'),
    (Div, '÷', '/'),
    (Max, '⎡', '^'),
    (Min, '⎣', '_'),
    (Mod, 'ᛁ'),
    (Pow, '*'),
    Log,
);

op!(
    ComparisonOp,
    (Equal, '='),
    (NotEqual, '≠', '='),
    (Less, '<'),
    (LessOrEqual, '≤', '<'),
    (Greater, '>'),
    (GreaterOrEqual, '≥', '>'),
);

op!(
    RuneOp,
    /// ?/Replicate
    (Fehu, 'ᚠ', 'f'),
    /// Transpose/Chunks
    (Uruz, 'ᚢ', 'u'),
    /// ?/Select
    (Ansuz, 'ᚨ', 'a'),
    /// ?/?
    (Kaunan, 'ᚲ', 'k'),
    /// ?/Drop
    (Gebo, 'ᚷ', 'g'),
    /// ?/Take
    (Naudiz, 'ᚾ', 'n'),
    /// Reverse/Rotate
    (Jera, 'ᛃ', 'j'),
    /// Join/Join To
    (Iwaz, 'ᛇ', 'A'),
    /// First/Index
    (Perth, 'ᛈ', 'p'),
    /// Range/Windows
    (Algiz, 'ᛉ', 'z'),
    /// Grade/?
    (Sowilo, 'ᛊ', 's'),
    /// Sort/?
    (Tiwaz, 'ᛏ', 't'),
    /// Identity/Right
    (Laguz, 'ᛚ', 'l'),
);

op!(OtherOp, (Match, '≡', ':'), (DoNotMatch, '≢', ';'));

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

impl From<OtherOp> for Op {
    fn from(o: OtherOp) -> Self {
        Op::Other(o)
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
    pub const fn from_glyph(glyph: char) -> Option<Self> {
        if let Some(p) = Pervasive::from_glyph(glyph) {
            Some(Op::Pervasive(p))
        } else if let Some(r) = RuneOp::from_glyph(glyph) {
            Some(Op::Rune(r))
        } else if let Some(o) = OtherOp::from_glyph(glyph) {
            Some(Op::Other(o))
        } else {
            None
        }
    }
    pub const fn from_escape(escape: char) -> Option<Self> {
        if let Some(p) = Pervasive::from_escape(escape) {
            Some(Op::Pervasive(p))
        } else if let Some(r) = RuneOp::from_escape(escape) {
            Some(Op::Rune(r))
        } else if let Some(o) = OtherOp::from_escape(escape) {
            Some(Op::Other(o))
        } else {
            None
        }
    }
}

impl Pervasive {
    pub const fn from_glyph(glyph: char) -> Option<Self> {
        if let Some(m) = MathOp::from_glyph(glyph) {
            Some(Pervasive::Math(m))
        } else if let Some(c) = ComparisonOp::from_glyph(glyph) {
            Some(Pervasive::Comparison(c))
        } else {
            None
        }
    }
    pub const fn from_escape(escape: char) -> Option<Self> {
        if let Some(m) = MathOp::from_escape(escape) {
            Some(Pervasive::Math(m))
        } else if let Some(c) = ComparisonOp::from_escape(escape) {
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
            Op::Other(o) => o.fmt(f),
        }
    }
}

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Op::Pervasive(p) => p.fmt(f),
            Op::Rune(r) => r.fmt(f),
            Op::Other(o) => o.fmt(f),
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
    /// Scan
    (Thurisaz, 'ᚦ', 'T'),
    /// Fold
    (Raido, 'ᚱ', 'r'),
    /// Table
    (Wunjo, 'ᚹ', 'w'),
    /// Each
    (Berkanan, 'ᛒ', 'b'),
    /// Constant
    (Ingwaz, 'ᛜ', 'N'),
    /// Flip
    (Othala, 'ᛟ', 'o'),
);

op!(
    RuneBinMod,
    /// Over
    (Haglaz, 'ᚻ', 'h'),
    /// Beside
    (Ehwaz, 'ᛖ', 'e'),
    /// ?
    (Mannaz, 'ᛗ', 'm'),
    /// Choose
    (Dagaz, 'ᛞ', 'd'),
    /// Catch
    (Stan, 'ᛥ', 'S'),
);
