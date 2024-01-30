use crate::ast::{ArgumentDeclarations, Location, Pidentifier, Identifier};
use std::collections::HashMap;
use num::BigInt;

pub fn add_command(code: &mut Vec<String>, command: &str) {
    code.push(String::from(command));
}

pub fn add_command_string(code: &mut Vec<String>, command: String) {
    code.push(command);
}

pub fn add_comment(code: &mut Vec<String>, comment: &str) {
    if code.len() > 0 {
        code[0] += " # ";
        code[0] += comment;
    }
}

pub enum DivisionType {
    Division,
    Modulo,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum RegisterState {
    Noise,
    Variable(Identifier),
    Constant(BigInt),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RegisterStates {
    pub registers: HashMap<Register, RegisterState>,
    next: Register,
}

impl RegisterStates {
    pub fn new() -> Self {
        let starting_states = HashMap::from([
            (Register::A, RegisterState::Noise),
            (Register::B, RegisterState::Noise),
            (Register::C, RegisterState::Noise),
            (Register::D, RegisterState::Noise),
            (Register::E, RegisterState::Noise),
            (Register::F, RegisterState::Noise),
            (Register::G, RegisterState::Noise),
            (Register::H, RegisterState::Noise),
        ]);
        return Self{registers: starting_states, next: Register::D};
    }

    fn next_register(&self, register: &Register) -> Register {
        return match register {
            Register::D => Register::E,
            Register::E => Register::F,
            Register::F => Register::G,
            Register::G => Register::H,
            Register::H => Register::D,
            _ => panic!("Invalid next register field"),
        };

    }

    pub fn get_next(&mut self) -> Register {
        /*
        for register in [Register::D, Register::E, Register::F, Register::G, Register::H] {
            if let RegisterState::Noise = self.registers.get(&register).unwrap() {
                self.next = self.next_register(&register);
                return register;
            }
        }
        */
        let current_register = self.next.clone();
        self.next = self.next_register(&current_register);
        return current_register;
    }

    pub fn scan(&self, id: &Identifier) -> Option<Register> {
        for (register, state) in self.registers.iter() {
            if *state == RegisterState::Variable(id.clone()) && register != &Register::A && register != &Register::B {
                return Some(register.clone());
            }
        }
        return None;
    }
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
    Constant(BigInt),
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
