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
fn move_value_code(register: &Register) -> String {
    if matches!(register, Register::A) {
        return String::new();
    } else {
        return "PUT ".to_owned() + register_to_string(register) + "\n";
    }
}

// create the specified Num (u64) value and store it in the register of choice
// TODO: efficient constant generation
fn translate_load_const(value: Num, register: &Register) -> String {
    let register_str = register_to_string(register);

    // reset the chosen register and progressively add ones until the value is reached

    return (0..value)
        .fold(String::from("RST ".to_owned() + register_str + "\n"), |mut code, _| {
            let inc_code = "INC ".to_owned() + register_str + "\n";
            code += &inc_code;
            code
        });
}

// fetch the address of a Pidentifier into the register of choice
fn translate_fetch_pid(varname: &Pidentifier, register: &Register, symbol_table: &SymbolTable) -> Result<String, TranslationError> {

    // check if the varname exists in the symbol table...

    if let Some(entry) = symbol_table.get(varname) {

        // ...and whether it's not an array...

        if let SymbolTableEntry::Var(var) = entry {

            // ...if so, then load its address into the specified register

            let comment = "# fetching ".to_owned() + varname + "'s address into register " + register_to_string(register) + "\n";
            return Ok(comment + &translate_load_const(var.memloc, register));
        } else {
            return Err(TranslationError::NotAnArray);
        }
    } else {
        return Err(TranslationError::NoVariableDefined);
    }
}

// fetch the address of a specified array element into the register of choice
// TODO: array bound checking
fn translate_fetch_arrnum(arrname: &Pidentifier, idx: Num, register: &Register, symbol_table: &SymbolTable) -> Result<String, TranslationError> {

    // check if the arrname exists in the symbol table...

    if let Some(entry) = symbol_table.get(arrname) {

        // ...and assert it is an array...

        if let SymbolTableEntry::Arr(arr) = entry {

            // ...if so, then load the address of the idx-th entry into the specified register

            let comment = "# fetching ".to_owned() + arrname + "[" + &idx.to_string() + "]'s address into register" + register_to_string(register) + "\n"; 
            return Ok(comment + &translate_load_const(arr.memloc + idx, register));
        } else {
            return Err(TranslationError::NoArrayIndex);
        }
    } else {
        return Err(TranslationError::NoVariableDefined);
    }
}

// fetch the address of an array entry with index equal to the value of the index variable and
// store it in the register of choice
// NOTICE: erases the contents of registers A and B
// TODO: replace register B with the register of choice?
fn translate_fetch_arrpid(arrname: &Pidentifier, idx_varname: &Pidentifier, register: &Register, symbol_table: &SymbolTable) -> Result<String, TranslationError> {
    let mut code = "# fetching ".to_owned() + arrname + "[" + idx_varname + "]'s address into register " + register_to_string(register) + "\n";

    // fetch the address of the indexing variable into register B...

    match translate_fetch_pid(idx_varname, &Register::B, symbol_table) {
        Ok(fetch_idx_var_code) => {
            code += &fetch_idx_var_code;

            // ...and then load its value into register A

            code += "LOAD b\n";

            // next, load the array address into register B
            
            match translate_fetch_arrnum(arrname, 0, &Register::B, symbol_table) {
                Ok(fetch_arr_code) => {
                    code += &fetch_arr_code;

                    // finally, add the address of the array in register B to the value of the
                    // indexing variable in register A to get the final address

                    let final_address_code = "Add b".to_owned() + " # calculating address of " + arrname + "[" + idx_varname + "]" + "\n";
                    code += &final_address_code;

                    // if the resulting address is to be stored in a register other than A, move it

                    code += &move_value_code(register);

                    return Ok(code);
                },
                Err(e) => return Err(e),
            }
        },
        Err(e) => return Err(e),
    }
}

// fetch the address of the specified Identifier into the register of choice
// NOTICE: erases the contents of registers A and B
fn translate_fetch(id: &Identifier, register: &Register, symbol_table: &SymbolTable) -> Result<String, TranslationError> {

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
fn translate_val(value: &Value, register: &Register, symbol_table: &SymbolTable) -> Result<String, TranslationError> {
    match value {
        Value::Number(num) => {

            // generate the const value in the chosen register

            let comment = "# generating constant ".to_owned() + &num.to_string() + " into register " + &register_to_string(register) + "\n";
            return Ok(comment + &translate_load_const(*num, register));
        },
        Value::Id(id) => {

            // fetch the address of the Identifier into register B...

            match translate_fetch(id, &Register::B, symbol_table) {
                Ok(mut code) => {

                    // ...and load its value into the specified register

                    let comment = "# loading ".to_owned() + &format!("{:?}", id) + "'s value into register " + &register_to_string(register) + "\n";
                    code += &comment;

                    code += "LOAD b\n";
                    code += &move_value_code(register);

                    return Ok(code);
                },
                Err(e) => return Err(e),
            }
        },
    }
}

// perform an add Expression for lhs and rhs Values and store the result in the register of choice
// NOTICE: erases the contents of registers A, B, C, and D
fn translate_add_expr(lhs: &Value, rhs: &Value, register: &Register, symbol_table: &SymbolTable) -> Result<String, TranslationError> {
    let mut code = "# ".to_owned() + &format!("{:?}", lhs) + " + " + &format!("{:?}", rhs) + "\n";

    // load the lhs value into register C...

    match translate_val(lhs, &Register::C, symbol_table) {
        Ok(lhs_code) => {
            code += &lhs_code;

            // ...and the rhs value into register D

            match translate_val(rhs, &Register::D, symbol_table) {
                Ok(rhs_code) => {
                    code += &rhs_code;

                    // then add them and move the result into the register of choice

                    let comment = "# performing addition; storing in register ".to_owned() + &register_to_string(register) + "\n";
                    code += &comment;
                    code += "GET c\n";
                    code += "ADD d\n";
                    code += &move_value_code(register);

                    return Ok(code);
                },
                Err(e) => return Err(e),
            }
        },
        Err(e) => return Err(e),
    }
}

// perform the sub Expression for lhs and rhs Values and store the result in the register of choice
// NOTICE: erases the contents of registers A, B, C and D
fn translate_sub_expr(lhs: &Value, rhs: &Value, register: &Register, symbol_table: &SymbolTable) -> Result<String, TranslationError> {
    let mut code = "# ".to_owned() + &format!("{:?}", lhs) + " - " + &format!("{:?}", rhs) + "\n";
    
    // load the lhs value into register C...

    match translate_val(lhs, &Register::C, symbol_table) {
        Ok(lhs_code) => {
            code += &lhs_code;

            // ...and the rhs value into register D

            match translate_val(rhs, &Register::D, symbol_table) {
                Ok(rhs_code) => {
                    code += &rhs_code;

                    // then subtract them and move the result into the register of choice

                    let comment = "# performing subtraction; storing in register ".to_owned() + &register_to_string(register) + "\n";
                    code += &comment;
                    code += "GET c\n";
                    code += "SUB d\n";
                    code += &move_value_code(register);

                    return Ok(code);
                },
                Err(e) => return Err(e),
            }
        },
        Err(e) => return Err(e),
    }
}

// perform the mul Expression for lhs and rhs Values and store the result in the register of choice
// TODO: finish this
// TODO: count lines for jumps!
// TODO: move end check just before final two shifts?
// TODO: optimise multiplication by a constant
// NOTICE: erases the contents of registers A, B, C and D
fn translate_mul_expr(lhs: &Value, rhs: &Value, register: &Register, symbol_table: &SymbolTable) -> Result<String, TranslationError> {

    // load the lhs value into register C, and rhs value into register D

    match translate_val(lhs, &Register::C, symbol_table) {
        Ok(lhs_code) => {
            match translate_val(rhs, &Register::D, symbol_table) {
                Ok(rhs_code) => {
                    let mut code = lhs_code + &rhs_code;

                    // then multiply them and move the result into the register of choice

                    code += "RST b\n";
                    code += "'mul_loop:\n";
                    code += "GET d\n";
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
                    
                    if !matches!(register, Register::B) {
                        code += "GET b\n";
                        let move_code = "PUT ".to_owned() + register_to_string(register) + "\n";
                        code += &move_code;
                    }

                    return Ok(code);
                },
                Err(e) => return Err(e),
            }
        },
        Err(e) => return Err(e),
    }
}

// TODO: implement
fn translate_div_expr(lhs: &Value, rhs: &Value, register: &Register, symbol_table: &SymbolTable) -> Result<String, TranslationError> {
    return Err(TranslationError::Temp);
}

// TODO: implement
fn translate_mod_expr(lhs: &Value, rhs: &Value, register: &Register, symbol_table: &SymbolTable) -> Result<String, TranslationError> {
    return Err(TranslationError::Temp);
}

// calculate the value of the specified Expression and store the result in the register of choice
// NOTICE: erases the contents of registers A, B, C and D
fn translate_expr(expr: &Expression, register: &Register, symbol_table: &SymbolTable) -> Result<String, TranslationError> {
    match expr {
        Expression::Val(value) =>
            return translate_val(value, register, symbol_table),
        Expression::Add(lhs, rhs) =>
            return translate_add_expr(lhs, rhs, register, symbol_table),
        Expression::Sub(lhs, rhs) =>
            return translate_sub_expr(lhs, rhs, register, symbol_table),
        Expression::Mul(lhs, rhs) =>
            return translate_mul_expr(lhs, rhs, register, symbol_table),
        Expression::Div(lhs, rhs) =>
            return translate_div_expr(lhs, rhs, register, symbol_table),
        Expression::Mod(lhs, rhs) =>
            return translate_mod_expr(lhs, rhs, register, symbol_table),
    }
}

// store the value of the rhs Expression at the address of the lhs Identifier
fn translate_assignment(id: &Identifier, expr: &Expression, symbol_table: &SymbolTable) -> Result<String, TranslationError> {

    // store the Expression value in register C

    match translate_expr(expr, &Register::C, symbol_table) {
        Ok(expr_code) => {

            // fetch the address of the Identifier in register B

            match translate_fetch(id, &Register::B, symbol_table) {
                Ok(fetch_code) => {
                    let mut code = expr_code + &fetch_code;

                    // store the calculated value under the specified address

                    let comment = "# storing the rhs value under the address of lhs\n";
                    code += &comment;
                    code += "GET c\n";
                    code += "STORE b\n";

                    return Ok(code);
                },
                Err(e) => return Err(e),
            }
        },
        Err(e) => return Err(e),
    }
}

// read user-inputted value and store it at the address of the Identifier
fn translate_read(id: &Identifier, symbol_table: &SymbolTable) -> Result<String, TranslationError> {
    
    // fetch the address of the Identifier in register B

    match translate_fetch(id, &Register::B, symbol_table) {
        Ok(fetch_code) => {
            let mut code = fetch_code;

            // read an input value into register A

            code += "READ\n";

            // store the read value under the address of the Identifier

            code += "STORE b\n";

            return Ok(code);
        },
        Err(e) => return Err(e),
    }
}

// write the specified Value on the output
fn translate_write(value: &Value, symbol_table: &SymbolTable) -> Result<String, TranslationError> {
    
    // fetch the Value into register A

    match translate_val(value, &Register::A, symbol_table) {
        Ok(val_code) => {
            let mut code = val_code;

            // write the value on the output

            code += "WRITE\n";

            return Ok(code);
        },
        Err(e) => return Err(e),
    }
}

// generate appropriate virtual machine code for the specified commands
fn translate_commands(commands: &Commands, symbol_table: &SymbolTable) -> Result<String, TranslationError> {
    let mut translated_code = String::new();
    for command in commands {

        // add a command comment

        let comment = "# --- ".to_owned() + &format!("{:?}", command) + " ---\n";
        translated_code += &comment;

        // translate the command

        match command {
            Command::Assignment(id, expr) => {
                match translate_assignment(id, expr, symbol_table) {
                    Ok(code) => translated_code += &code,
                    Err(e) => return Err(e),
                }
            },
            Command::Read(id) => {
                match translate_read(id, symbol_table) {
                    Ok(code) => translated_code += &code,
                    Err(e) => return Err(e),
                }
            }
            Command::Write(value) => {
                match translate_write(value, symbol_table) {
                    Ok(code) => translated_code += &code,
                    Err(e) => return Err(e),
                }
            }
            _ => return Err(TranslationError::Temp)
        }
    }
    return Ok(translated_code);
}

pub fn translate(ast: ProgramAll) -> Result<String, TranslationError> {
    let mut symbol_table = SymbolTable::new();
    
    match malloc(0, &ast.main.declarations, &mut symbol_table) {
        Ok(_next_mem_byte) => {
            println!("Symbol table: {:?}", symbol_table);
            match translate_commands(&ast.main.commands, &symbol_table) {
                Ok(translated_code) => return Ok(translated_code + "HALT\n"),
                Err(e) => return Err(e),
            }
        },
        Err(e) => return Err(e),
    }
}
