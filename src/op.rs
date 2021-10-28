use std::fmt;

use crate::error::CompileError;

pub trait Visit<S> {
    type Input;
    type Output;
    type Error;
    fn visit_un(&self, inner: Self::Input, state: &mut S) -> Result<Self::Output, Self::Error>;
    fn visit_bin(
        &self,
        left: Self::Input,
        right: Self::Output,
        state: &mut S,
    ) -> Result<Self::Output, Self::Error>;
}

macro_rules! op {
    ($(($name:ident, $glyph:literal)),* $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub enum Op {
            $($name),*
        }

        impl Op {
            pub const fn glyph(&self) -> char {
                match self {
                    $(Op::$name => $glyph,)*
                }
            }
            pub fn from_glyph(glyph: char) -> Option<Self> {
                match glyph {
                    $($glyph => Some(Op::$name),)*
                    _ => None,
                }
            }
        }

        impl fmt::Display for Op {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                match self {
                    $(Op::$name => $glyph.fmt(f),)*
                }
            }
        }
    }
}

op!(
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
