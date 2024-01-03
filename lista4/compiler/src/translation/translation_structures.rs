pub fn add_command(code: &mut Vec<String>, command: &str) {
    code.push(String::from(command));
}

pub fn add_command_string(code: &mut Vec<String>, command: String) {
    code.push(command);
}

pub fn add_comment(code: &mut Vec<String>, comment: &str) {
    code[0] +=  " # ";
    code[0] += comment;
}

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
    NoSuchVariable,
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
    pub is_ref: bool,
}

impl Variable {
    pub fn new(ml: u64, ir: bool) -> Self {
        return Self{memloc: ml, is_ref: ir};
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReturnLocation {
    pub memloc: u64,
}

impl ReturnLocation {
    pub fn new(ml: u64) -> Self {
        return Self{memloc: ml};
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SymbolTableEntry {
    Var(Variable),
    Arr(Array),
    Ret(ReturnLocation),
}

pub type SymbolTable = HashMap<String, SymbolTableEntry>;
