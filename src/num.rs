use std::{cmp::Ordering, f64, fmt, num::ParseFloatError, ops::*, str::FromStr};

pub fn modulus<T>(a: T, b: T) -> T
where
    T: Copy + Rem<Output = T> + Add<Output = T>,
{
    (a % b + b) % b
}

/// Numbers in can be either integers or floating point.
/// All operations on integers, except for division, produce integers.
/// Floating point numbers infect integers, turning them into floating
/// point as well. Floating point numbers can be turned back into integers
/// with the [`Num::floor`], [`Num::ceil`], and [`Num::round`] methods.
#[derive(Clone, Copy)]
pub enum Num {
    /// Integers
    Int(i64),
    /// FLoating point
    Float(f64),
}

impl Default for Num {
    fn default() -> Self {
        Num::Int(0)
    }
}

impl Num {
    pub const INFINIFY: Self = Num::Float(f64::INFINITY);
    pub const NAN: Self = Num::Float(f64::NAN);
    pub const PI: Self = Num::Float(f64::consts::PI);
    pub const E: Self = Num::Float(f64::consts::E);
    pub fn is_infinite(&self) -> bool {
        match self {
            Num::Int(_) => false,
            Num::Float(f) => f.is_infinite(),
        }
    }
    /// Convert to the next lowest integer
    pub fn floor(self) -> Self {
        match self {
            Num::Int(i) => Num::Int(i),
            Num::Float(f) => Num::Int(f.floor() as i64),
        }
    }
    /// Convert to the next highest integer
    pub fn ceil(self) -> Self {
        match self {
            Num::Int(i) => Num::Int(i),
            Num::Float(f) => Num::Int(f.ceil() as i64),
        }
    }
    /// Round to the nearest integer
    pub fn round(self) -> Self {
        match self {
            Num::Int(i) => Num::Int(i),
            Num::Float(f) => Num::Int(f.round() as i64),
        }
    }
    /// Get the absolute value
    pub fn abs(self) -> Self {
        match self {
            Num::Int(i) => Num::Int(i.abs()),
            Num::Float(f) => Num::Float(f.abs()),
        }
    }
    /// Get the sign
    #[allow(clippy::comparison_chain)]
    pub fn sign(self) -> Self {
        if self == 0 {
            0i64
        } else if self > 0 {
            1
        } else {
            -1
        }
        .into()
    }
    /// Raise the number to a power
    ///
    /// Raising an integer to the power of a non-negative integer will produce another integer.
    /// All other combinations will return a floating point number
    pub fn pow(self, power: Num) -> Self {
        match (self, power) {
            (Num::Int(a), Num::Int(b)) if b >= 0 => Num::Int(a.saturating_pow(b as u32)),
            (Num::Int(a), Num::Int(b)) => Num::Float((a as f64).powf(b as f64)),
            (Num::Int(a), Num::Float(b)) => Num::Float((a as f64).powf(b)),
            (Num::Float(a), Num::Int(b)) => Num::Float(a.powf(b as f64)),
            (Num::Float(a), Num::Float(b)) => Num::Float(a.powf(b)),
        }
    }
    pub fn log(self, base: Num) -> Self {
        f64::from(self).log(base.into()).into()
    }
    /// Get the true modulus of the number with some radix
    pub fn modulus(self, radix: Num) -> Self {
        self.binary_op(radix, modulus, modulus)
    }
    /// Perform a binary operation on this number and another
    pub fn binary_op<I, F>(self, other: Num, int: I, float: F) -> Num
    where
        I: FnOnce(i64, i64) -> i64,
        F: FnOnce(f64, f64) -> f64,
    {
        let (a, b) = match (self, other) {
            (Num::Int(a), Num::Int(b)) => return Num::Int(int(a, b)),
            (Num::Int(a), Num::Float(b)) => (a as f64, b),
            (Num::Float(a), Num::Int(b)) => (a, b as f64),
            (Num::Float(a), Num::Float(b)) => (a, b),
        };
        Num::Float(float(a, b))
    }
    /// Perform a binary operation on this number and another
    pub fn binary_op_ref<I, F, T>(&self, other: &Num, int: I, float: F) -> T
    where
        I: FnOnce(&i64, &i64) -> T,
        F: FnOnce(&f64, &f64) -> T,
    {
        match (self, other) {
            (Num::Int(a), Num::Int(b)) => int(a, b),
            (Num::Int(a), Num::Float(b)) => float(&(*a as f64), b),
            (Num::Float(a), Num::Int(b)) => float(a, &(*b as f64)),
            (Num::Float(a), Num::Float(b)) => float(a, b),
        }
    }
    pub fn string_format(&self, string: &str) -> String {
        if string.contains('e') || string.contains('E') {
            string.replace('-', "‾")
        } else {
            let mut s = String::new();
            let n = *self;
            if n < Num::Int(0) {
                s.push('‾');
            }
            let n_string = n.abs().to_string();
            let mut parts = n_string.split('.');
            let left = parts.next().unwrap();
            let right = parts.next();
            let underscores = n.abs() >= 10000;
            for (i, c) in left.chars().enumerate() {
                let i = left.len() - i - 1;
                s.push(c);
                if underscores && i > 0 && i % 3 == 0 {
                    s.push('_');
                }
            }
            if let Some(right) = right {
                s.push('.');
                for (i, c) in right.chars().enumerate() {
                    s.push(c);
                    if i > 0 && i % 3 == 2 {
                        s.push('_');
                    }
                }
            }
            s
        }
    }
}

impl From<u8> for Num {
    fn from(i: u8) -> Self {
        Num::Int(i as i64)
    }
}

impl From<i64> for Num {
    fn from(i: i64) -> Self {
        Num::Int(i)
    }
}

impl From<f64> for Num {
    fn from(f: f64) -> Self {
        Num::Float(f)
    }
}

impl From<usize> for Num {
    fn from(u: usize) -> Self {
        Num::Int(u as i64)
    }
}

impl From<u32> for Num {
    fn from(u: u32) -> Self {
        Num::Int(u as i64)
    }
}

impl From<Num> for i64 {
    fn from(num: Num) -> Self {
        match num {
            Num::Int(i) => i,
            Num::Float(f) => f as i64,
        }
    }
}

impl From<Num> for f64 {
    fn from(num: Num) -> Self {
        match num {
            Num::Int(i) => i as f64,
            Num::Float(f) => f,
        }
    }
}

impl From<Num> for u32 {
    fn from(num: Num) -> Self {
        match num {
            Num::Int(i) => i as u32,
            Num::Float(f) => f as u32,
        }
    }
}

impl fmt::Debug for Num {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for Num {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self < &Num::Int(0) {
            write!(f, "‾")?;
        }
        if self.is_infinite() {
            write!(f, "∞")
        } else {
            match self.abs() {
                Num::Int(i) => i.fmt(f),
                Num::Float(i) => i.fmt(f),
            }
        }
    }
}

impl Add for Num {
    type Output = Self;
    fn add(self, other: Self) -> Self::Output {
        self.binary_op(other, i64::saturating_add, f64::add)
    }
}

impl Sub for Num {
    type Output = Self;
    fn sub(self, other: Self) -> Self::Output {
        self.binary_op(other, i64::saturating_sub, f64::sub)
    }
}

impl Mul for Num {
    type Output = Self;
    fn mul(self, other: Self) -> Self::Output {
        self.binary_op(other, i64::saturating_mul, f64::mul)
    }
}

impl Div for Num {
    type Output = Self;
    fn div(self, other: Self) -> Self::Output {
        let (a, b) = match (self, other) {
            (_, b) if b == 0 => return Num::NAN,
            (Num::Int(a), Num::Int(b)) if a % b == 0 => return Num::Int(a / b),
            (Num::Int(a), Num::Int(b)) => (a as f64, b as f64),
            (Num::Int(a), Num::Float(b)) => (a as f64, b),
            (Num::Float(a), Num::Int(b)) => (a, b as f64),
            (Num::Float(a), Num::Float(b)) => (a, b),
        };
        Num::Float(a / b)
    }
}

trait NumCmp {
    fn cmp(&self, other: &Self) -> Ordering;
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl NumCmp for i64 {
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(self, other)
    }
}

impl NumCmp for f64 {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.is_nan(), other.is_nan()) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (false, false) => self.partial_cmp(other).unwrap(),
        }
    }
}

impl PartialEq for Num {
    fn eq(&self, other: &Self) -> bool {
        self.binary_op_ref(other, NumCmp::eq, NumCmp::eq)
    }
}

impl PartialEq<i64> for Num {
    fn eq(&self, other: &i64) -> bool {
        match self {
            Num::Int(i) => i == other,
            Num::Float(f) => NumCmp::eq(f, &(*other as f64)),
        }
    }
}

impl PartialEq<f64> for Num {
    fn eq(&self, other: &f64) -> bool {
        match self {
            Num::Int(i) => NumCmp::eq(&(*i as f64), other),
            Num::Float(f) => NumCmp::eq(f, other),
        }
    }
}

impl Eq for Num {}

impl PartialOrd for Num {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Num {
    fn cmp(&self, other: &Self) -> Ordering {
        self.binary_op_ref(other, NumCmp::cmp, NumCmp::cmp)
    }
}

impl PartialOrd<i64> for Num {
    fn partial_cmp(&self, other: &i64) -> Option<Ordering> {
        Some(match self {
            Num::Int(i) => Ord::cmp(i, other),
            Num::Float(f) => NumCmp::cmp(f, &(*other as f64)),
        })
    }
}

impl FromStr for Num {
    type Err = ParseFloatError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(if let Ok(i) = s.parse::<i64>() {
            Num::Int(i)
        } else {
            Num::Float(s.parse()?)
        })
    }
}

impl Neg for Num {
    type Output = Self;
    fn neg(self) -> Self::Output {
        match self {
            Num::Int(i) => Num::Int(-i),
            Num::Float(f) => Num::Float(-f),
        }
    }
}
