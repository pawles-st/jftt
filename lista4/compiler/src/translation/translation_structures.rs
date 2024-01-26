use crate::ast::{ArgumentDeclarations, Location, Pidentifier};
use std::collections::HashMap;

pub fn add_command(code: &mut Vec<String>, command: &str) {
    code.push(String::from(command));
}

pub fn add_command_string(code: &mut Vec<String>, command: String) {
    code.push(command);
}

pub fn add_comment(code: &mut Vec<String>, comment: &str) {
    code[0] += " # ";
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
    NoSuchVariable(Location, Pidentifier),
    NoSuchProcedure(Location, Pidentifier),
    RepeatedDeclaration(Location, Pidentifier),
    NotAnArray(Location, Pidentifier),
    NoArrayIndex(Location, Pidentifier),
    ArrayExpected(Location, Pidentifier),
    VariableExpected(Location, Pidentifier),
    RecurrenceNotAllowed(Location, Pidentifier),
    InvalidNumberOfArguments(Location, Pidentifier),
    UninitialisedVariable(Location, Pidentifier),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValueHeld {
    Uninitialised,
    Dynamic,
    Constant(u64),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Array {
    pub memloc: u64,
    pub len: u64,
    pub value: ValueHeld,
    pub is_ref: bool,
}

impl Array {
    pub fn new(ml: u64, l: u64, v: ValueHeld, ir: bool) -> Self {
        return Self{memloc: ml, len: l, value: v, is_ref: ir};
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Variable {
    pub memloc: u64,
    pub value: ValueHeld,
    pub is_ref: bool,
}

impl Variable {
    pub fn new(ml: u64, v: ValueHeld, ir: bool) -> Self {
        return Self{memloc: ml, value: v, is_ref: ir};
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

pub struct ProcedureInfo {
    pub args_decl: ArgumentDeclarations,
    pub code_line_number: usize,
    pub mem_addr: u64,
}

impl ProcedureInfo {
    pub fn new(ad: ArgumentDeclarations, cln: usize, ma: u64) -> Self {
        return Self{args_decl: ad, code_line_number: cln, mem_addr: ma};
    }
}

pub type FunctionTable = HashMap<String, ProcedureInfo>;
