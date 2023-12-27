use std::cell::Cell;
use std::ops::Add;
use std::ops::Sub;
use std::ops::Mul;
use std::ops::Div;

pub const P: Cell<u64> = Cell::new(1234576);

#[derive(Debug)]
pub struct Modulo {
    value: u64,
}

impl Modulo {
    pub fn new(v: u64) -> Self {
        return Self{value: v % P.get()};
    }
}

impl Add for Modulo {
    type Output = Self;

    fn add(self, r: Modulo) -> Self {
        return Self::new(self.value + r.value);
    }
}

impl Sub for Modulo {
    type Output = Self;

    fn sub(self, r: Self) -> Self {
        return self + Modulo::neg(r);
    }
}

impl Mul for Modulo {
    type Output = Self;

    fn mul(self, r: Self) -> Self {
        return Self::new(self.value * r.value);
    }
}

impl Div for Modulo {
    type Output = Self;

    fn div(self, r: Self) -> Self {
        return self * Modulo::inv(r);
    }
}

fn extended_euclid(a: u64, b: u64) -> (u64, u64, u64) {
    if a == 0 {
        return (b, 0, 1);
    } else {
        let (d, x, y) = extended_euclid(b % a, a);
        return (d, y - (b / a) * x, x);
    }
}

impl Modulo {
    pub fn neg(n: Self) -> Self {
        return Self::new(P.get() - n.value);
    }
    pub fn inv(n: Self) -> Self {
        let (_, x, _) = extended_euclid(n.value, P.get());
        return Self::new(x);
    }
}
