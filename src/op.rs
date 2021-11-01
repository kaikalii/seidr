use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Op {
    Pervasive(Pervasive),
    Rune(Rune),
}

impl From<Pervasive> for Op {
    fn from(p: Pervasive) -> Self {
        Op::Pervasive(p)
    }
}

impl From<Rune> for Op {
    fn from(r: Rune) -> Self {
        Op::Rune(r)
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
        } else if let Some(r) = Rune::from_glyph(glyph) {
            Some(Op::Rune(r))
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

op!(
    Pervasive,
    // Math
    (Add, '+'),
    (Sub, '-'),
    (Mul, '×'),
    (Div, '÷'),
    // Comparison
    (Equal, '='),
    (NotEqual, '≠'),
    (Less, '<'),
    (LessOrEqual, '≤'),
    (Greater, '>'),
    (GreaterOrEqual, '≥'),
);

op!(
    Rune,
    // Runes
    (Fehu, 'ᚠ'),
    (Uruz, 'ᚢ'),
    (Thurisaz, 'ᚦ'),
    (Ansuz, 'ᚨ'),
    (Raido, 'ᚱ'),
    (Kaunan, 'ᚲ'),
    (Gebo, 'ᚷ'),
    (Wunjo, 'ᚹ'),
    (Haglaz, 'ᚻ'),
    (Naudiz, 'ᚾ'),
    (Isaz, 'ᛁ'),
    (Jera, 'ᛃ'),
    (Iwaz, 'ᛇ'),
    (Perth, 'ᛈ'),
    (Algiz, 'ᛉ'),
    (Sowilo, 'ᛊ'),
    (Tiwaz, 'ᛏ'),
    (Berkanan, 'ᛒ'),
    (Ehwaz, 'ᛖ'),
    (Mannaz, 'ᛗ'),
    (Laguz, 'ᛚ'),
    (Ingwaz, 'ᛜ'),
    (Othala, 'ᛟ'),
    (Dagaz, 'ᛞ'),
);
