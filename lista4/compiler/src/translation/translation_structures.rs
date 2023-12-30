#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Register {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
}

pub fn register_to_string<'a>(r: &'a Register) -> &'a str {
    match r {
        Register::A => return "a",
        Register::B => return "b",
        Register::C => return "c",
        Register::D => return "d",
        Register::E => return "e",
        Register::F => return "f",
        Register::G => return "g",
        Register::H => return "h",
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TranslationError {
    NoVariableDefined,
    RepeatedDeclaration,
    NotAnArray,
    NoArrayIndex,
    Temp,
}

use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Array {
    pub memloc: u64,
    pub len: u64,
}

impl Array {
    pub fn new(ml: u64, l: u64) -> Self {
        return Self{memloc: ml, len: l};
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Variable {
    pub memloc: u64,
}

impl Variable {
    pub fn new(ml: u64) -> Self {
        return Self{memloc: ml};
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SymbolTableEntry {
    Var(Variable),
    Arr(Array),
}

pub type SymbolTable = HashMap<String, SymbolTableEntry>;
