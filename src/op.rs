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
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    }
}

op!(
    (Add, '+'),
    (Sub, '-'),
    (Mul, '×'),
    (Div, '÷'),
    (Less, '<'),
    (LessOrEqual, '≤'),
    (Greater, '>'),
    (GreaterOrEqual, '≥'),
    (Equal, '='),
    (NotEqual, '≠'),
);
