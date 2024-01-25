pub type Location = (usize, usize);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProgramAll {
    pub procedures: Procedures,
    pub main: Main,
}

impl ProgramAll {
    pub fn new(p: Procedures, m: Main) -> Self {
        return Self{procedures: p, main: m};
    }
}

pub type Procedures = Vec<Procedure>;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Procedure {
    pub proc_head: ProcHead,
    pub declarations: Declarations,
    pub commands: Commands,
    pub location: Location,
}

impl Procedure {
    pub fn new(ph: ProcHead, d: Declarations, c: Commands, l: Location) -> Self {
        return Self{proc_head: ph, declarations: d, commands: c, location: l};
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Main {
    pub declarations: Declarations,
    pub commands: Commands,
    pub location: Location
}

impl Main {
    pub fn new(d: Declarations, c: Commands, l: Location) -> Self {
        return Self{declarations: d, commands: c, location: l};
    }
}

pub type Commands = Vec<Command>;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Command {
    Assignment(Identifier, Expression, Location),
    IfElse(Condition, Commands, Commands, Location),
    If(Condition, Commands, Location),
    While(Condition, Commands, Location),
    Repeat(Commands, Condition, Location),
    ProcedureCall(ProcCall, Location),
    Read(Identifier, Location),
    Write(Value, Location),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProcHead {
    pub name: Pidentifier,
    pub args_decl: ArgumentDeclarations,
}

impl ProcHead {
    pub fn new(p: Pidentifier, a: ArgumentDeclarations) -> Self {
        return Self{name: p, args_decl: a};
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProcCall {
    pub name: Pidentifier,
    pub args: Arguments,
}

impl ProcCall {
    pub fn new(p: Pidentifier, a: Arguments) -> Self {
        return Self{name: p, args: a};
    }
}

pub type Declarations = Vec<Declaration>;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Declaration {
    Var(Pidentifier),
    Arr(Pidentifier, Num),
}

pub type ArgumentDeclarations = Vec<ArgumentDeclaration>;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum ArgumentDeclaration {
    Var(Pidentifier),
    Arr(Pidentifier),
}

pub type Arguments = Vec<Pidentifier>;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Expression {
    Val(Value),
    Add(Value, Value),
    Sub(Value, Value),
    Mul(Value, Value),
    Div(Value, Value),
    Mod(Value, Value),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Condition {
    Equal(Value, Value),
    NotEqual(Value, Value),
    Greater(Value, Value),
    Lesser(Value, Value),
    GreaterOrEqual(Value, Value),
    LesserOrEqual(Value, Value),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Value {
    Number(Num),
    Id(Identifier),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Identifier {
    Pid(Pidentifier),
    ArrNum(Pidentifier, Num),
    ArrPid(Pidentifier, Pidentifier),
}

pub type Pidentifier = String;

pub type Num = u64;
