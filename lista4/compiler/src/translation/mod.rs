use crate::ast::*;
use translation_structures::*;

mod translation_structures;
mod tests;

// create an entry in the symbol table for each variable and array declaration
fn malloc(mut curr_mem_byte: u64, decls: &Declarations, symbol_table: &mut SymbolTable) -> Result<u64, TranslationError> {
    for decl in decls {
        match decl {
            Declaration::Var(pid) => {
                if symbol_table.contains_key(pid) {
                    return Err(TranslationError::RepeatedDeclaration);
                } else {
                    symbol_table.insert(pid.to_string(), SymbolTableEntry::Var(Variable::new(curr_mem_byte)));
                    curr_mem_byte += 1;
                }
            },
            Declaration::Arr(pid, len) => {
                if symbol_table.contains_key(pid) {
                    return Err(TranslationError::RepeatedDeclaration);
                } else {
                    symbol_table.insert(pid.to_string(), SymbolTableEntry::Arr(Array::new(curr_mem_byte, *len)));
                    curr_mem_byte += *len;
                }
            },
        }
    }
    return Ok(curr_mem_byte);
}

// move the value from register A into the register of choice
fn move_value_code(register: &Register) -> Vec<String> {
    if matches!(register, Register::A) {
        return Vec::new();
    } else {
        return vec!["PUT ".to_owned() + register_to_string(register)];
    }
}

// create the specified Num (u64) value and store it in the register of choice
// TODO: efficient constant generation
fn translate_load_const(value: Num, register: &Register) -> Vec<String> {
    let register_str = register_to_string(register);

    // reset the chosen register and progressively add ones until the value is reached

    return (0..value)
        .fold(vec![String::from("RST ".to_owned() + register_str)], |mut code, _| {
            let inc_code = "INC ".to_owned() + register_str;
            code.push(inc_code);
            code
        });
}

// fetch the address of a Pidentifier into the register of choice
fn translate_fetch_pid(varname: &Pidentifier, register: &Register, symbol_table: &SymbolTable) -> Result<Vec<String>, TranslationError> {

    // check if the varname exists in the symbol table...

    if let Some(entry) = symbol_table.get(varname) {

        // ...and whether it's not an array...

        if let SymbolTableEntry::Var(var) = entry {

            let mut code = Vec::new();

            // ...if so, then load its address into the specified register

            let mut pid_address_code = translate_load_const(var.memloc, register);
            code.append(&mut pid_address_code);

            let comment = "fetching ".to_owned() + varname + "'s address into register " + register_to_string(register);
            add_comment(&mut code, &comment);

            return Ok(code);
        } else {
            return Err(TranslationError::NotAnArray);
        }
    } else {
        return Err(TranslationError::NoSuchVariable);
    }
}

// fetch the address of a specified array element into the register of choice
// TODO: array bound checking
fn translate_fetch_arrnum(arrname: &Pidentifier, idx: Num, register: &Register, symbol_table: &SymbolTable) -> Result<Vec<String>, TranslationError> {

    // check if the arrname exists in the symbol table...

    if let Some(entry) = symbol_table.get(arrname) {

        // ...and assert it is an array...

        if let SymbolTableEntry::Arr(arr) = entry {

            let mut code = Vec::new();

            // ...if so, then load the address of the idx-th entry into the specified register
           
            let mut arrnum_address_code = translate_load_const(arr.memloc + idx, register);
            code.append(&mut arrnum_address_code);

            let comment = "fetching ".to_owned() + arrname + "[" + &idx.to_string() + "]'s address into register" + register_to_string(register);
            add_comment(&mut code, &comment);

            return Ok(code);
        } else {
            return Err(TranslationError::NoArrayIndex);
        }
    } else {
        return Err(TranslationError::NoSuchVariable);
    }
}

// fetch the address of an array entry with index equal to the value
// of the indexing variable and store it in the register of choice
// NOTICE: erases the contents of registers A and B
// TODO: replace register B with the register of choice?
fn translate_fetch_arrpid(arrname: &Pidentifier, idx_varname: &Pidentifier, register: &Register, symbol_table: &SymbolTable) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    // fetch the address of the indexing variable into register B...

    let mut fetch_idx_var_code = translate_fetch_pid(idx_varname, &Register::B, symbol_table)?;
    code.append(&mut fetch_idx_var_code);
    
    let comment = "fetching ".to_owned() + arrname + "[" + idx_varname + "]'s address into register " + register_to_string(register);
    add_comment(&mut code, &comment);

    // ...and then load its value into register A

    add_command(&mut code, "LOAD b");

    // next, load the array address into register B
   
    let mut fetch_arr_code = translate_fetch_arrnum(arrname, 0, &Register::B, symbol_table)?;
    code.append(&mut fetch_arr_code);

    // finally, add the address of the array in register B to the value of the
    // indexing variable in register A to get the final address
    
    let mut offset_code = Vec::new();
    add_command(&mut offset_code, "ADD b");

    let comment = "calculating address of ".to_owned() + arrname + "[" + idx_varname + "]";
    add_comment(&mut offset_code, &comment);

    code.append(&mut offset_code);

    // if the resulting address is to be stored in a register other than A, move it

    code.append(&mut move_value_code(register));

    return Ok(code);
}

// fetch the address of the specified Identifier into the register of choice
// NOTICE: erases the contents of registers A and B
fn translate_fetch(id: &Identifier, register: &Register, symbol_table: &SymbolTable) -> Result<Vec<String>, TranslationError> {

    // execute the appropriate fetch code based on the Identifier type

    match id {
        Identifier::Pid(varname) => 
            return translate_fetch_pid(varname, register, symbol_table),
        Identifier::ArrNum(arrname, idx) =>
            return translate_fetch_arrnum(arrname, *idx, register, symbol_table),
        Identifier::ArrPid(arrname, idx_varname) =>
            return translate_fetch_arrpid(arrname, idx_varname, register, symbol_table),
    }
}

// fetch the specified Value into the register of choice
// NOTICE: erases the contents of registers A and B
fn translate_val(value: &Value, register: &Register, symbol_table: &SymbolTable) -> Result<Vec<String>, TranslationError> {
    match value {
        Value::Number(num) => {

            let mut code = Vec::new();

            // generate the const value in the chosen register

            code.append(&mut translate_load_const(*num, register));

            let comment = "generating constant ".to_owned() + &num.to_string() + " into register " + &register_to_string(register);
            add_comment(&mut code, &comment);

            return Ok(code);
        },
        Value::Id(id) => {

            // fetch the address of the Identifier into register B...

            let mut code = Vec::new();

            let mut fetch_id_code = translate_fetch(id, &Register::B, symbol_table)?;
            code.append(&mut fetch_id_code);

            // ...and load its value into the specified register

            let mut store_code = Vec::new();
            add_command(&mut store_code, "LOAD b");

            let comment = " loading ".to_owned() + &format!("{:?}", id) + "'s value into register " + &register_to_string(register);
            add_comment(&mut store_code, &comment);

            code.append(&mut store_code);

            code.append(&mut move_value_code(register));
            
            return Ok(code);
        },
    }
}

// perform an add Expression for lhs and rhs Values and store the result in the register of choice
// NOTICE: erases the contents of registers A, B, C, and D
fn translate_add_expr(lhs: &Value, rhs: &Value, register: &Register, symbol_table: &SymbolTable) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();
    
    // load the lhs value into register C...

    let mut lhs_code = translate_val(lhs, &Register::C, symbol_table)?;
    code.append(&mut lhs_code);
    
    let comment = format!("{:?}", lhs) + " + " + &format!("{:?}", rhs);
    add_comment(&mut code, &comment);

    // ...and the rhs value into register D

    let mut rhs_code = translate_val(rhs, &Register::D, symbol_table)?;
    code.append(&mut rhs_code);

    // then add them and move the result into the register of choice

    let mut addition_code = Vec::new();
    add_command(&mut addition_code, "GET c");
    add_command(&mut addition_code, "ADD d");

    let comment = "performing addition; storing in register ".to_owned() + &register_to_string(register);
    add_comment(&mut addition_code, &comment);

    code.append(&mut addition_code);
    
    code.append(&mut move_value_code(register));

    return Ok(code);
}

// perform the sub Expression for lhs and rhs Values and store the result in the register of choice
// NOTICE: erases the contents of registers A, B, C and D
fn translate_sub_expr(lhs: &Value, rhs: &Value, register: &Register, symbol_table: &SymbolTable) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();
    
    // load the lhs value into register C...

    let mut lhs_code = translate_val(lhs, &Register::C, symbol_table)?;
    code.append(&mut lhs_code);

    let comment = format!("{:?}", lhs) + " - " + &format!("{:?}", rhs);
    add_comment(&mut code, &comment);

    // ...and the rhs value into register D

    let mut rhs_code = translate_val(rhs, &Register::D, symbol_table)?;
    code.append(&mut rhs_code);

    // then subtract them and move the result into the register of choice

    let mut subtraction_code = Vec::new();
    add_command(&mut subtraction_code, "GET c");
    add_command(&mut subtraction_code, "SUB d");

    let comment = "performing subtraction; storing in register ".to_owned() + &register_to_string(register);
    add_comment(&mut subtraction_code, &comment);
    
    code.append(&mut subtraction_code);

    code.append(&mut move_value_code(register));

    return Ok(code);
}

// perform the mul Expression for lhs and rhs Values and store the result in the register of choice
// TODO: finish this
// TODO: count lines for jumps!
// TODO: move end check just before final two shifts?
// TODO: optimise multiplication by a constant
// NOTICE: erases the contents of registers A, B, C and D
fn translate_mul_expr(lhs: &Value, rhs: &Value, register: &Register, symbol_table: &SymbolTable, curr_line: usize) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    // load the lhs value into register C, and rhs value into register D

    let mut lhs_code = translate_val(lhs, &Register::C, symbol_table)?;
    code.append(&mut lhs_code);

    let mut rhs_code = translate_val(rhs, &Register::D, symbol_table)?;
    code.append(&mut rhs_code);

    // then multiply them and move the result into the register of choice

    // register B will hold the result, so reset it

    add_command(&mut code, "RST b");
    add_command(&mut code, "GET d"); // mul_loop_line
    let end_loop_line = (curr_line + 13).to_string();
    add_command_string(&mut code, "JZERO ".to_owned() + &end_loop_line);
    add_command(&mut code, "SHR d");
    add_command(&mut code, "SHL d");
    add_command(&mut code, "SUB d");
    let after_add_line = (curr_line + 10).to_string();
    add_command_string(&mut code, "JZERO ".to_owned() + &after_add_line);
    add_command(&mut code, "GET b");
    add_command(&mut code, "ADD c");
    add_command(&mut code, "PUT b");
    add_command(&mut code, "SHL c"); // after_add_line
    add_command(&mut code, "SHR d");
    let mul_loop_line = (curr_line + 1).to_string();
    add_command_string(&mut code, "JUMP ".to_owned() + &mul_loop_line);
    // end_loop_line

    /*
    code += "JZERO 'end\n";
    code += "SHR d\n";
    code += "SHL d\n";
    code += "SUB d\n";
    code += "JZERO after_add\n";
    code += "GET b\n";
    code += "ADD c\n";
    code += "PUT b\n";
    code += "'after_add:\n";
    code += "SHL c\n";
    code += "SHR d\n";
    code += "JUMP 'mul_loop\n";
    code += "'end:\n";
    */
    
    // if the result is to be stored in a register other than B, move it

    if !matches!(register, Register::B) {
        add_command(&mut code, "GET b");
        let move_code = "PUT ".to_owned() + register_to_string(register);
        add_command_string(&mut code, move_code);
    }

    return Ok(code);
}

// TODO: implement
fn translate_div_expr(lhs: &Value, rhs: &Value, register: &Register, symbol_table: &SymbolTable, curr_line: usize) -> Result<Vec<String>, TranslationError> {
    return Err(TranslationError::Temp);
}

// TODO: implement
fn translate_mod_expr(lhs: &Value, rhs: &Value, register: &Register, symbol_table: &SymbolTable, curr_line: usize) -> Result<Vec<String>, TranslationError> {
    return Err(TranslationError::Temp);
}

// calculate the value of the specified Expression and store the result in the register of choice
// NOTICE: erases the contents of registers A, B, C and D
fn translate_expr(expr: &Expression, register: &Register, symbol_table: &SymbolTable, curr_line: usize) -> Result<Vec<String>, TranslationError> {
    match expr {
        Expression::Val(value) =>
            return translate_val(value, register, symbol_table),
        Expression::Add(lhs, rhs) =>
            return translate_add_expr(lhs, rhs, register, symbol_table),
        Expression::Sub(lhs, rhs) =>
            return translate_sub_expr(lhs, rhs, register, symbol_table),
        Expression::Mul(lhs, rhs) =>
            return translate_mul_expr(lhs, rhs, register, symbol_table, curr_line),
        Expression::Div(lhs, rhs) =>
            return translate_div_expr(lhs, rhs, register, symbol_table, curr_line),
        Expression::Mod(lhs, rhs) =>
            return translate_mod_expr(lhs, rhs, register, symbol_table, curr_line),
    }
}

// store the value of the rhs Expression at the address of the lhs Identifier
fn translate_assignment(id: &Identifier, expr: &Expression, symbol_table: &SymbolTable, curr_line: usize) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    // store the Expression value in register C

    let mut expr_code = translate_expr(expr, &Register::C, symbol_table, curr_line)?;
    code.append(&mut expr_code);
    
    // fetch the address of the Identifier into register B

    let mut id_fetch_code = translate_fetch(id, &Register::B, symbol_table)?;
    code.append(&mut id_fetch_code);

    // store the calculated value under the specified address

    let mut store_code = Vec::new();
    add_command(&mut store_code, "GET c");
    add_command(&mut store_code, "STORE b");

    let comment = "storing the rhs value under the address of lhs".to_owned();
    add_comment(&mut store_code, &comment);
    
    code.append(&mut store_code);

    return Ok(code);
}

// call 
fn translate_proc_call(name: &Pidentifier, args: &Arguments, symbol_table: &SymbolTable, curr_line: usize) -> Result<Vec<String>, TranslationError> {
    return Err(TranslationError::Temp);
}

// read user-inputted value and store it at the address of the Identifier
fn translate_read(id: &Identifier, symbol_table: &SymbolTable) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();
    
    // fetch the address of the Identifier in register B

    let mut id_fetch_code = translate_fetch(id, &Register::B, symbol_table)?;
    code.append(&mut id_fetch_code);

    // read an input value into register A

    add_command(&mut code, "READ");

    // store the read value under the address of the Identifier

    add_command(&mut code, "STORE");

    return Ok(code);
}

// write the specified Value on the output
fn translate_write(value: &Value, symbol_table: &SymbolTable) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();
    
    // fetch the Value into register A

    let mut val_code = translate_val(value, &Register::A, symbol_table)?;
    code.append(&mut val_code);

    // write the value on the output

    add_command(&mut code, "WRITE");

    return Ok(code);
}

// generate appropriate virtual machine code for the specified commands
fn translate_commands(commands: &Commands, symbol_table: &SymbolTable, curr_line: usize) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    for command in commands {

        // translate the command

        match command {
            Command::Assignment(id, expr) => {
                let mut command_code = translate_assignment(id, expr, symbol_table, curr_line + code.len())?;
                let comment = "--- ".to_owned() + &format!("{:?}", command) + " ---";
                add_comment(&mut command_code, &comment);
                code.append(&mut command_code);
            },
            Command::ProcedureCall(proc_call) => {
                let mut command_code = translate_proc_call(proc_call.name, proc_call.args, symbol_table, curr_line + code.len())?;
                let comment = "--- ".to_owned() + &format!("{:?}", command) + " ---";
                add_comment(&mut command_code, &comment);
                code.append(&mut command_code);
            }
            Command::Read(id) => {
                let mut command_code = translate_read(id, symbol_table)?;
                let comment = "--- ".to_owned() + &format!("{:?}", command) + " ---";
                add_comment(&mut command_code, &comment);
                code.append(&mut command_code);
            }
            Command::Write(value) => {
                let mut command_code = translate_write(value, symbol_table)?;
                let comment = "--- ".to_owned() + &format!("{:?}", command) + " ---";
                add_comment(&mut command_code, &comment);
                code.append(&mut command_code);
            }
            _ => return Err(TranslationError::Temp)
        }
    }
    return Ok(code);
}

fn translate_procedure(procedure: &Procedure) -> Result<Vec<String>, TranslationError> {
    
}

pub fn translate(ast: ProgramAll) -> Result<Vec<String>, TranslationError> {
    //let mut code = Vec::new();

    //let mut function_table = None;

    for procedure in ast.procedures {
        translate_procedure(&procedure);
    }
    
    let mut symbol_table = SymbolTable::new();

    // add all Main declaration to the symbol table
    
    let _next_mem_byte = malloc(0, &ast.main.declarations, &mut symbol_table)?;
    println!("Symbol table: {:?}", symbol_table);

    // translate the code inside Main

    let mut code = translate_commands(&ast.main.commands, &symbol_table, 0)?;

    for i in 0..code.len() {
        if code[i].starts_with("#") {
            panic!("Error: comment at the beginning of line");
        }
        if code[i].ends_with("\n") {
            panic!("No newline symbol at the end of line");
        }
    }

    // if the code is correct, halt the program

    add_command(&mut code, "HALT");

    return Ok(code);
}
