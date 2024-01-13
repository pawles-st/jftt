use std::iter::zip;
use crate::ast::*;
use translation_structures::*;

mod translation_structures;
mod tests;

// create an entry in the function table for the proc_head
fn malloc_proc(proc_head: &ProcHead, function_table: &mut FunctionTable, code_line_number: usize, mem_addr: u64) -> Result<(), TranslationError> {
    let proc_name = proc_head.name.to_owned();

    // check if the procedure name is unique...

    if function_table.contains_key(&proc_name) {
        return Err(TranslationError::RepeatedDeclaration);
    }

    // ...if so, add the proc_name and args_decl to the function_table

    function_table.insert(proc_name, ProcedureInfo::new(proc_head.args_decl.clone(), code_line_number, mem_addr));

    Ok(())
}

// create an entry in the symbol table for each variable and array reference
fn malloc_args(mut curr_mem_byte: u64, decls: &ArgumentDeclarations, symbol_table: &mut SymbolTable) -> Result<u64, TranslationError> {
    for decl in decls {
        match decl {
            ArgumentDeclaration::Var(pid) => {
                if symbol_table.contains_key(pid) {
                    return Err(TranslationError::RepeatedDeclaration);
                } else {
                    symbol_table.insert(pid.to_owned(), SymbolTableEntry::Var(Variable::new(curr_mem_byte, true)));
                    curr_mem_byte += 1;
                }
            },
            ArgumentDeclaration::Arr(pid) => {
                if symbol_table.contains_key(pid) {
                    return Err(TranslationError::RepeatedDeclaration);
                } else {
                    symbol_table.insert(pid.to_owned(), SymbolTableEntry::Arr(Array::new(curr_mem_byte, 0, true)));
                    curr_mem_byte += 1;
                }
            }
        }
    }
    return Ok(curr_mem_byte);
}

// create an entry in the symbol table for each variable and array declaration
fn malloc(mut curr_mem_byte: u64, decls: &Declarations, symbol_table: &mut SymbolTable) -> Result<u64, TranslationError> {
    for decl in decls {
        match decl {
            Declaration::Var(pid) => {
                if symbol_table.contains_key(pid) {
                    return Err(TranslationError::RepeatedDeclaration);
                } else {
                    symbol_table.insert(pid.to_owned(), SymbolTableEntry::Var(Variable::new(curr_mem_byte, false)));
                    curr_mem_byte += 1;
                }
            },
            Declaration::Arr(pid, len) => {
                if symbol_table.contains_key(pid) {
                    return Err(TranslationError::RepeatedDeclaration);
                } else {
                    symbol_table.insert(pid.to_owned(), SymbolTableEntry::Arr(Array::new(curr_mem_byte, *len, false)));
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
// NOTICE: erases the contents of registers A and B
// TODO: replace register B with the register of choice?
fn translate_fetch_pid(varname: &Pidentifier, register: &Register, symbol_table: &SymbolTable) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    // check if the varname exists in the symbol table...

    if let Some(entry) = symbol_table.get(varname) {

        // ...and whether it's not an array

        if let SymbolTableEntry::Var(var) = entry {

            // next, check whether the variable holds a reference...

            if var.is_ref {

                // ...if so, load the reference's address into register B

                let mut ref_address_code = translate_load_const(var.memloc, &Register::B);
                code.append(&mut ref_address_code);

                let comment = varname.to_owned() + " IS ref; indirectly fetching address into register " + register_to_string(register);
                add_comment(&mut code, &comment);

                // next, load the value stored under the reference's address
                // into register A - that is the original variables's address

                add_command(&mut code, "LOAD b");

                // if the resulting address is to be stored in a register other than A, move it

                code.append(&mut move_value_code(register));
            } else {

                // ..otherwise, load the address directly into the specified register

                let mut pid_address_code = translate_load_const(var.memloc, register);
                code.append(&mut pid_address_code);

                let comment = varname.to_owned() + " is NOT ref; directly fetching address into register " + register_to_string(register);
                add_comment(&mut code, &comment);
            }
        } else {
            return Err(TranslationError::NotAnArray);
        }
    } else {
        return Err(TranslationError::NoSuchVariable);
    }
    return Ok(code);
}

// fetch the address of a specified array element into the register of choice
// NOTICE: erases the contents of registers A and B
// TODO: replace register B with the register of choice?
// TODO: array bound checking
fn translate_fetch_arrnum(arrname: &Pidentifier, idx: Num, register: &Register, symbol_table: &SymbolTable) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    // check if the arrname exists in the symbol table...

    if let Some(entry) = symbol_table.get(arrname) {

        // ...and assert it is an array

        if let SymbolTableEntry::Arr(arr) = entry {

            // next, check whether the variable holds a reference...

            if arr.is_ref {

                // if so, load the reference's address into register B

                let mut ref_address_code = translate_load_const(arr.memloc, &Register::B);
                code.append(&mut ref_address_code);

                let comment = arrname.to_owned() + "IS array ref; indirectly fetching address into register " + register_to_string(register);
                add_comment(&mut code, &comment);

                // next, load the value stored under the reference's address
                // into register A - that is the array's beginning address

                add_command(&mut code, "LOAD b");

                // load the array index into register B

                let mut idx_load_code = translate_load_const(idx, &Register::B);
                code.append(&mut idx_load_code);

                // add the two together to get the final address

                add_command(&mut code, "ADD b");

                // if the resulting address is to be stored in a register other than A, move it

                code.append(&mut move_value_code(register));
            } else {
                
                // ..otherwise, load the address directly into the specified register

                let mut arrnum_address_code = translate_load_const(arr.memloc + idx, register);
                code.append(&mut arrnum_address_code);

                let comment = arrname.to_owned() + "is NOT array ref; directly fetching address into register " + register_to_string(register);
                add_comment(&mut code, &comment);
            }
        } else {
            return Err(TranslationError::NoArrayIndex);
        }
    } else {
        return Err(TranslationError::NoSuchVariable);
    }
    return Ok(code);
}

// fetch the address of an array entry with index equal to the value
// of the indexing variable and store it in the register of choice
// NOTICE: erases the contents of registers A, B and E
// TODO: replace register B with the register of choice?
fn translate_fetch_arrpid(arrname: &Pidentifier, idx_varname: &Pidentifier, register: &Register, symbol_table: &SymbolTable) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    // fetch the address of the indexing variable into register B...

    let mut fetch_idx_var_code = translate_fetch_pid(idx_varname, &Register::B, symbol_table)?;
    code.append(&mut fetch_idx_var_code);
    
    let comment = "fetching ".to_owned() + arrname + "[" + idx_varname + "]'s address into register " + register_to_string(register);
    add_comment(&mut code, &comment);

    // ...then load its value into register A...

    add_command(&mut code, "LOAD b");

    // ...and temporarily move it to register E

    add_command(&mut code, "PUT e");

    // next, load the array address into register B
   
    let mut fetch_arr_code = translate_fetch_arrnum(arrname, 0, &Register::B, symbol_table)?;
    code.append(&mut fetch_arr_code);

    // move the indexing variable's value back to register A

    add_command(&mut code, "GET e");

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
// NOTICE: erases the contents of registers A, B and E
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
// NOTICE: erases the contents of registers A, B and E
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
// NOTICE: erases the contents of registers A, B, C, D and E
fn translate_add_expr(lhs: &Value, rhs: &Value, register: &Register, symbol_table: &SymbolTable) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();
    
    // load the lhs value into register C...

    let mut lhs_code = translate_val(lhs, &Register::C, symbol_table)?;
    code.append(&mut lhs_code);

    // ...and the rhs value into register D

    let mut rhs_code = translate_val(rhs, &Register::D, symbol_table)?;
    code.append(&mut rhs_code);
    
    let comment = format!("{:?}", lhs) + " + " + &format!("{:?}", rhs);
    add_comment(&mut code, &comment);

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
// NOTICE: erases the contents of registers A, B, C, D and E
fn translate_sub_expr(lhs: &Value, rhs: &Value, register: &Register, symbol_table: &SymbolTable) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();
    
    // load the lhs value into register C...

    let mut lhs_code = translate_val(lhs, &Register::C, symbol_table)?;
    code.append(&mut lhs_code);

    // ...and the rhs value into register D

    let mut rhs_code = translate_val(rhs, &Register::D, symbol_table)?;
    code.append(&mut rhs_code);

    let comment = format!("{:?}", lhs) + " - " + &format!("{:?}", rhs);
    add_comment(&mut code, &comment);

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
// TODO: move end check just before final two shifts?
// TODO: optimise multiplication by a constant
// NOTICE: erases the contents of registers A, B, C, D, and E
fn translate_mul_expr(lhs: &Value, rhs: &Value, register: &Register, symbol_table: &SymbolTable, mut curr_line: usize) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    // load the lhs value into register C, and rhs value into register D

    let mut lhs_code = translate_val(lhs, &Register::C, symbol_table)?;
    code.append(&mut lhs_code);

    let mut rhs_code = translate_val(rhs, &Register::D, symbol_table)?;
    code.append(&mut rhs_code);

    let comment = format!("{:?}", lhs) + " * " + &format!("{:?}", rhs);
    add_comment(&mut code, &comment);
    
    curr_line += code.len();

    // then multiply them and move the result into the register of choice

    let mut multiplication_code = Vec::new();

    // register B will hold the result, so reset it

    add_command(&mut multiplication_code, "RST b");
    add_command(&mut multiplication_code, "GET d"); // mul_loop_line
    let end_loop_line = (curr_line + 13).to_string();
    add_command_string(&mut multiplication_code, "JZERO ".to_owned() + &end_loop_line);
    add_command(&mut multiplication_code, "SHR d");
    add_command(&mut multiplication_code, "SHL d");
    add_command(&mut multiplication_code, "SUB d");
    let after_add_line = (curr_line + 10).to_string();
    add_command_string(&mut multiplication_code, "JZERO ".to_owned() + &after_add_line);
    add_command(&mut multiplication_code, "GET b");
    add_command(&mut multiplication_code, "ADD c");
    add_command(&mut multiplication_code, "PUT b");
    add_command(&mut multiplication_code, "SHL c"); // after_add_line
    add_command(&mut multiplication_code, "SHR d");
    let mul_loop_line = (curr_line + 1).to_string();
    add_command_string(&mut multiplication_code, "JUMP ".to_owned() + &mul_loop_line);
    // end_loop_line

    code.append(&mut multiplication_code);

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
// NOTICE: erases the contents of registers A, B, C, D and E
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

// return from the procedure to the caller
fn translate_return(symbol_table: &SymbolTable) -> Vec<String> {
    let mut code = Vec::new();

    // assert the return location object has been stored in the symbol table...

    if let Some(ret) = symbol_table.get(".return") {

        // ...and that it is of a correct type

        if let SymbolTableEntry::Ret(return_location) = ret {

            // if so, load the return address...

            let mut ret_addr_code = translate_load_const(return_location.memloc, &Register::B);
            code.append(&mut ret_addr_code);
            add_command(&mut code, "LOAD b");

            let comment = "return to the caller";
            add_comment(&mut code, comment);

            // ...and jump to the line number equal to this value

            add_command(&mut code, "JUMPR a");
        } else {
            panic!("Expected return address object in the symbol table, found {:?}", ret);
        }
    } else {
        panic!("Expected a return address object in the symbol table, but none was found");
    }
    return code;
}


fn translate_store_var_reference(arg_memloc: u64, is_ref: bool, store_memloc: u64) -> Vec<String> {
    let mut code = Vec::new();

    if is_ref {

        // if the variable is a reference, first load the reference's address into register B...

        let mut fetch_ref_code = translate_load_const(arg_memloc, &Register::B);
        code.append(&mut fetch_ref_code);

        // ...and then fetch the value stored under it (original var's address) into register A

        add_command(&mut code, "LOAD b");

        // load the store memory location into register B

        let mut fetch_store_code = translate_load_const(store_memloc, &Register::B);
        code.append(&mut fetch_store_code);

        // store the original variable's address

        add_command(&mut code, "STORE b");
    } else {

        // if the variable isn't a reference, load the variable's address into register A

        let mut fetch_var_code = translate_load_const(arg_memloc, &Register::A);
        code.append(&mut fetch_var_code);

        // load the store memory location into register B

        let mut fetch_store_code = translate_load_const(store_memloc, &Register::B);
        code.append(&mut fetch_store_code);

        // store the variable's address

        add_command(&mut code, "STORE b");
    }
    return code;
}

// call a procedure with given arguments
fn translate_proc_call(name: &Pidentifier, args: &Arguments, symbol_table: &SymbolTable, function_table: &FunctionTable, curr_line: usize, curr_proc: Option<&Pidentifier>) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    // check for a recursive call

    if let Some(curr_proc_name) = curr_proc {
        if name == curr_proc_name {
            return Err(TranslationError::RecurrenceNotAllowed);
        }
    }

    // fetch the destination procedure information from the function table

    if let Some(proc_info) = function_table.get(name) {

        // check if the number of arguments matches the number
        // of parameters of the destination procedure

        if args.len() != proc_info.args_decl.len() {
            return Err(TranslationError::InvalidNumberOfArguments);
        }

        // check if the type of each argument matches
        // the type of the destination procedure parameter

        for (arg_no, (arg_name, arg_decl)) in zip(args, &proc_info.args_decl).enumerate() {
            if let Some(arg_entry) = symbol_table.get(arg_name) {

                // check type equality

                if matches!(arg_decl, ArgumentDeclaration::Var{..}) {
                    if let SymbolTableEntry::Var(arg) = arg_entry { // both are variables

                        // store the variable reference

                        let mut store_addr_code = translate_store_var_reference(arg.memloc, arg.is_ref, proc_info.mem_addr + 1 + arg_no as u64);
                        code.append(&mut store_addr_code);
                    } else {
                        return Err(TranslationError::VariableExpected);
                    }
                } else if matches!(arg_decl, ArgumentDeclaration::Arr{..}) {
                    if let SymbolTableEntry::Arr(arg) = arg_entry { // both are arrays
                        
                        // store the variable reference

                        let mut store_addr_code = translate_store_var_reference(arg.memloc, arg.is_ref, proc_info.mem_addr + 1 + arg_no as u64);
                        code.append(&mut store_addr_code);
                    } else {
                        return Err(TranslationError::ArrayExpected);
                    }
                } else {
                    panic!("Invalid argument declaration type in the symbol table");
                }
            } else {
                return Err(TranslationError::NoSuchVariable);
            }
        }

        // load the return's storage address...

        let mut store_addr = translate_load_const(proc_info.mem_addr, &Register::B);
        code.append(&mut store_addr);

        // ...and store the return address there

        let mut return_addr_offset = translate_load_const(4, &Register::C);
        code.append(&mut return_addr_offset);

        add_command(&mut code, "STRK a");
        add_command(&mut code, "ADD c");
        add_command(&mut code, "STORE b");

        // jump to the address that begins the procedure

        add_command_string(&mut code, "JUMP ".to_owned() + &(proc_info.code_line_number).to_string());
        
    } else {
        return Err(TranslationError::NoSuchProcedure);
    }
    return Ok(code);
}

fn translate_equal(lhs: &Value, rhs: &Value, symbol_table: &SymbolTable) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    // load the lhs value into register C...

    let mut lhs_code = translate_val(lhs, &Register::C, symbol_table)?;
    code.append(&mut lhs_code);

    // ...and the rhs value into register D

    let mut rhs_code = translate_val(rhs, &Register::D, symbol_table)?;
    code.append(&mut rhs_code);

    let comment = "condition".to_owned() + &format!("{:?}", lhs) + " = " + &format!("{:?}", rhs) + "";
    add_comment(&mut code, &comment);

    // then check for equality of the two values
    
    let mut comparison_code = Vec::new();
    
    add_command(&mut comparison_code, "GET c");
    add_command(&mut comparison_code, "SUB d");
    add_command(&mut comparison_code, "JPOS "); // blank jump

    add_command(&mut comparison_code, "GET d");
    add_command(&mut comparison_code, "SUB c");
    add_command(&mut comparison_code, "JPOS "); // blank jump

    return Ok(code);
}

fn translate_not_equal(lhs: &Value, rhs: &Value, symbol_table: &SymbolTable) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    // load the lhs value into register C...

    let mut lhs_code = translate_val(lhs, &Register::C, symbol_table)?;
    code.append(&mut lhs_code);

    // ...and the rhs value into register D...

    let mut rhs_code = translate_val(rhs, &Register::D, symbol_table)?;
    code.append(&mut rhs_code);

    let comment = "condition".to_owned() + &format!("{:?}", lhs) + " != " + &format!("{:?}", rhs) + "";
    add_comment(&mut code, &comment);

    // then check for difference of the two values

    let mut comparison_code = Vec::new();

    add_command(&mut comparison_code, "GET c");
    add_command(&mut comparison_code, "SUB d");
    add_command(&mut comparison_code, "PUT e");

    add_command(&mut comparison_code, "GET d");
    add_command(&mut comparison_code, "SUB c");
    add_command(&mut comparison_code, "ADD e");
    add_command(&mut comparison_code, "JPOS "); // blank jump
    
    return Ok(code);
}

fn translate_greater(lhs: &Value, rhs: &Value, symbol_table: &SymbolTable) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    // load the lhs value into register C...

    let mut lhs_code = translate_val(lhs, &Register::C, symbol_table)?;
    code.append(&mut lhs_code);

    // ...and the rhs value into register D...

    let mut rhs_code = translate_val(rhs, &Register::D, symbol_table)?;
    code.append(&mut rhs_code);

    let comment = "condition".to_owned() + &format!("{:?}", lhs) + " > " + &format!("{:?}", rhs) + "";
    add_comment(&mut code, &comment);

    // then check if lhs > rhs
    
    let mut comparison_code = Vec::new();

    add_command(&mut comparison_code, "GET c");
    add_command(&mut comparison_code, "SUB d");
    add_command(&mut comparison_code, "JZERO "); // blank jump
    code.append(&mut comparison_code);

    return Ok(code);
}

fn translate_lesser(lhs: &Value, rhs: &Value, symbol_table: &SymbolTable) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    // load the lhs value into register C...

    let mut lhs_code = translate_val(lhs, &Register::C, symbol_table)?;
    code.append(&mut lhs_code);

    // ...and the rhs value into register D...

    let mut rhs_code = translate_val(rhs, &Register::D, symbol_table)?;
    code.append(&mut rhs_code);

    let comment = "condition".to_owned() + &format!("{:?}", lhs) + " < " + &format!("{:?}", rhs) + "";
    add_comment(&mut code, &comment);

    // then check if lhs > rhs

    let mut comparison_code = Vec::new();

    add_command(&mut comparison_code, "GET d");
    add_command(&mut comparison_code, "SUB c");
    add_command(&mut comparison_code, "JZERO "); // blank jump
    code.append(&mut comparison_code);

    return Ok(code);
}

fn translate_greater_or_equal(lhs: &Value, rhs: &Value, symbol_table: &SymbolTable) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    // load the lhs value into register C...

    let mut lhs_code = translate_val(lhs, &Register::C, symbol_table)?;
    code.append(&mut lhs_code);

    // ...and the rhs value into register D...

    let mut rhs_code = translate_val(rhs, &Register::D, symbol_table)?;
    code.append(&mut rhs_code);

    let comment = "condition".to_owned() + &format!("{:?}", lhs) + " >= " + &format!("{:?}", rhs) + "";
    add_comment(&mut code, &comment);

    // then check if lhs > rhs

    let mut comparison_code = Vec::new();

    add_command(&mut comparison_code, "GET d");
    add_command(&mut comparison_code, "SUB c");
    add_command(&mut comparison_code, "JPOS "); // blank jump
    code.append(&mut comparison_code);

    return Ok(code);
}

fn translate_lesser_or_equal(lhs: &Value, rhs: &Value, symbol_table: &SymbolTable) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    // load the lhs value into register C...

    let mut lhs_code = translate_val(lhs, &Register::C, symbol_table)?;
    code.append(&mut lhs_code);

    // ...and the rhs value into register D...

    let mut rhs_code = translate_val(rhs, &Register::D, symbol_table)?;
    code.append(&mut rhs_code);

    let comment = "condition".to_owned() + &format!("{:?}", lhs) + " <= " + &format!("{:?}", rhs) + "";
    add_comment(&mut code, &comment);

    // then check if lhs > rhs
    
    let mut comparison_code = Vec::new();

    add_command(&mut comparison_code, "GET c");
    add_command(&mut comparison_code, "SUB d");
    add_command(&mut comparison_code, "JPOS "); // blank jump
    code.append(&mut comparison_code);

    return Ok(code);
}

fn translate_condition(condition: &Condition, symbol_table: &SymbolTable) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    match condition {

        // translate the corresponding condition

        Condition::Equal(lhs, rhs) => {
            let mut condition_code = translate_equal(lhs, rhs, symbol_table)?;
            code.append(&mut condition_code);
        },
        Condition::NotEqual(lhs, rhs) => {
            let mut condition_code = translate_not_equal(lhs, rhs, symbol_table)?;
            code.append(&mut condition_code);
        },
        Condition::Greater(lhs, rhs) => {
            let mut condition_code = translate_greater(lhs, rhs, symbol_table)?;
            code.append(&mut condition_code);
        },
        Condition::Lesser(lhs, rhs) => {
            let mut condition_code = translate_lesser(lhs, rhs, symbol_table)?;
            code.append(&mut condition_code);
        },
        Condition::GreaterOrEqual(lhs, rhs) => {
            let mut condition_code = translate_greater_or_equal(lhs, rhs, symbol_table)?;
            code.append(&mut condition_code);
        },
        Condition::LesserOrEqual(lhs, rhs) => {
            let mut condition_code = translate_lesser_or_equal(lhs, rhs, symbol_table)?;
            code.append(&mut condition_code);
        },
    }

    return Ok(code);
}

fn translate_if(condition: &Condition, commands: &Commands, symbol_table: &SymbolTable, function_table: &FunctionTable, curr_line: usize, curr_proc: Option<&Pidentifier>) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    // translate the condition code

    let mut condition_code = translate_condition(condition, symbol_table)?;

    // translate the if commands

    let mut commands_code = translate_commands(commands, symbol_table, function_table, curr_line, curr_proc)?;

    // fill the blank jumps in condition code

    let end_jump_line = curr_line + condition_code.len() + commands_code.len();

    if matches!(condition, Condition::Equal{..}) {

        // equality condition requires filling an extra jump

        let blank_jump_idx = condition_code.len() - 4;
        condition_code[blank_jump_idx] += &(end_jump_line.to_string());
    }

    let blank_jump_idx = condition_code.len() - 1;
    condition_code[blank_jump_idx] += &(end_jump_line.to_string());

    // join the partial codes

    code.append(&mut condition_code);
    code.append(&mut commands_code);

    return Ok(code);
}

fn translate_if_else(condition: &Condition, if_commands: &Commands, else_commands: &Commands, symbol_table: &SymbolTable, function_table: &FunctionTable, curr_line: usize, curr_proc: Option<&Pidentifier>) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    // translate the condition code

    let mut condition_code = translate_condition(condition, symbol_table)?;

    // translate the if commands

    let mut if_commands_code = translate_commands(if_commands, symbol_table, function_table, curr_line, curr_proc)?;

    // translate the else commands

    let mut else_commands_code = translate_commands(else_commands, symbol_table, function_table, curr_line, curr_proc)?;

    // jump at the end of the if_commands block

    let end_jump_line = curr_line + condition_code.len() + if_commands_code.len() + else_commands_code.len() + 1;
    add_command_string(&mut if_commands_code, "JUMP ".to_owned() + &(end_jump_line.to_string()));

    // fill the blank jumps in condition code

    let else_jump_line = curr_line + condition_code.len() + if_commands_code.len();

    if matches!(condition, Condition::Equal{..}) {

        // equality condition requires filling an extra jump

        let blank_jump_idx = condition_code.len() - 4;
        condition_code[blank_jump_idx] += &(else_jump_line.to_string());
    }

    let blank_jump_idx = condition_code.len() - 1;
    condition_code[blank_jump_idx] += &(else_jump_line.to_string());

    // join the partial codes

    code.append(&mut condition_code);
    code.append(&mut if_commands_code);
    code.append(&mut else_commands_code);

    return Ok(code);
}

fn translate_while(condition: &Condition, commands: &Commands, symbol_table: &SymbolTable, function_table: &FunctionTable, curr_line: usize, curr_proc: Option<&Pidentifier>) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    // translate the condition code

    let mut condition_code = translate_condition(condition, symbol_table)?;

    // translate the loop commands

    let mut commands_code = translate_commands(commands, symbol_table, function_table, curr_line, curr_proc)?;
    
    // jump to the beginning of the loop at the end of commands block

    add_command_string(&mut code, "JUMP ".to_owned() + &(curr_line.to_string()));

    // fill the blank jumps in condition code

    let end_jump_line = curr_line + condition_code.len() + commands_code.len();

    if matches!(condition, Condition::Equal{..}) {

        // equality condition requires filling an extra jump

        let blank_jump_idx = condition_code.len() - 4;
        condition_code[blank_jump_idx] += &(end_jump_line.to_string());
    }

    let blank_jump_idx = condition_code.len() - 1;
    condition_code[blank_jump_idx] += &(end_jump_line.to_string());

    // join the partial codes

    code.append(&mut condition_code);
    code.append(&mut commands_code);

    return Ok(code);
}

fn translate_repeat(commands: &Commands, condition: &Condition, symbol_table: &SymbolTable, function_table: &FunctionTable, curr_line: usize, curr_proc: Option<&Pidentifier>) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    // translate the condition code

    let mut condition_code = translate_condition(condition, symbol_table)?;

    // translate the loop commands

    let mut commands_code = translate_commands(commands, symbol_table, function_table, curr_line, curr_proc)?;

    // fill the blank jumps in condition code

    if matches!(condition, Condition::Equal{..}) {

        // equality condition requires filling an extra jump

        let blank_jump_idx = condition_code.len() - 4;
        condition_code[blank_jump_idx] += &(curr_line.to_string());
    }

    let blank_jump_idx = condition_code.len() - 1;
    condition_code[blank_jump_idx] += &(curr_line.to_string());

    // join the partial codes

    code.append(&mut commands_code);
    code.append(&mut condition_code);

    return Ok(code);
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

    add_command(&mut code, "STORE b");

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
fn translate_commands(commands: &Commands, symbol_table: &SymbolTable, function_table: &FunctionTable, curr_line: usize, curr_proc: Option<&Pidentifier>) -> Result<Vec<String>, TranslationError> {
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
            Command::If(condition, commands) => {
                let mut command_code = translate_if(condition, commands, symbol_table, function_table, curr_line + code.len(), curr_proc)?;
                let comment = "--- ".to_owned() + &format!("{:?}", command) + " ---";
                add_comment(&mut command_code, &comment);
                code.append(&mut command_code);
            }
            Command::IfElse(condition, if_commands, else_commands) => {
                let mut command_code = translate_if_else(condition, if_commands, else_commands, symbol_table, function_table, curr_line + code.len(), curr_proc)?;
                let comment = "--- ".to_owned() + &format!("{:?}", command) + " ---";
                add_comment(&mut command_code, &comment);
                code.append(&mut command_code);
            }
            Command::While(condition, commands) => {
                let mut command_code = translate_while(condition, commands, symbol_table, function_table, curr_line + code.len(), curr_proc)?;
                let comment = "--- ".to_owned() + &format!("{:?}", command) + " ---";
                add_comment(&mut command_code, &comment);
                code.append(&mut command_code);
            }
            Command::Repeat(commands, condition) => {
                let mut command_code = translate_repeat(commands, condition, symbol_table, function_table, curr_line + code.len(), curr_proc)?;
                let comment = "--- ".to_owned() + &format!("{:?}", command) + " ---";
                add_comment(&mut command_code, &comment);
                code.append(&mut command_code);
            }
            Command::ProcedureCall(proc_call) => {
                let mut command_code = translate_proc_call(&proc_call.name, &proc_call.args, symbol_table, function_table, curr_line + code.len(), curr_proc)?;
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

fn translate_procedure(procedure: &Procedure, function_table: &mut FunctionTable, mut curr_mem_byte: u64, curr_line: usize) -> Result<(Vec<String>, u64), TranslationError> {
    let mut code = Vec::new();

    // add the procedure to the function table

    malloc_proc(&procedure.proc_head, function_table, curr_line, curr_mem_byte)?;

    // create the procedure's symbol table

    let mut symbol_table = SymbolTable::new();

    // insert the return address object into the symbol table

    symbol_table.insert(".return".to_owned(), SymbolTableEntry::Ret(ReturnLocation::new(curr_mem_byte)));
    curr_mem_byte += 1;

    // allocate memory for the argument references and procedure declarations

    let curr_mem_byte = malloc_args(curr_mem_byte, &procedure.proc_head.args_decl, &mut symbol_table)?;
    let next_mem_byte = malloc(curr_mem_byte, &procedure.declarations, &mut symbol_table)?;
    //println!("{} Symbol table: {:?}", &procedure.proc_head.name, symbol_table);

    // translate the procedure commands

    let mut proc_code = translate_commands(&procedure.commands, &symbol_table, &function_table, curr_line, Some(&procedure.proc_head.name))?;
    code.append(&mut proc_code);

    // attach return code

    let mut ret_code = translate_return(&symbol_table);
    code.append(&mut ret_code);

    return Ok((code, next_mem_byte));
}

fn translate_main(main: &Main, function_table: &FunctionTable, curr_mem_byte: u64, curr_line: usize) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    // create main's symbol table

    let mut symbol_table = SymbolTable::new();

    // allocate memory for the declarations
    
    let _next_mem_byte = malloc(curr_mem_byte, &main.declarations, &mut symbol_table)?;
    //println!("Main Symbol table: {:?}", symbol_table);

    // translate the Main commands

    let mut main_code = translate_commands(&main.commands, &symbol_table, function_table, curr_line, None)?;
    code.append(&mut main_code);

    return Ok(code);

}

// TODO: check variable initialisation
pub fn translate(ast: ProgramAll) -> Result<Vec<String>, TranslationError> {
    let mut code = vec!["JUMP <MAIN ADDRESS>".to_owned()];

    let mut function_table = FunctionTable::new();
    let mut curr_mem_byte = 0;

    // translate the procedures into code

    for procedure in ast.procedures {

        // translate the the procedure

        let (mut proc_code, next_mem_byte) = translate_procedure(&procedure, &mut function_table, curr_mem_byte, code.len())?;
        add_comment(&mut proc_code, &procedure.proc_head.name);
        code.append(&mut proc_code);

        // update the location of the next free memory byte

        curr_mem_byte = next_mem_byte;
    }

    // jump to main

    let main_jump_code = "JUMP ".to_owned() + &code.len().to_string();
    code[0] = main_jump_code;

    // translate main into code

    let mut main_code = translate_main(&ast.main, &function_table, curr_mem_byte, code.len())?;
    add_comment(&mut main_code, ">>> Main <<<");
    code.append(&mut main_code);

    // some simple verifications of the code

    for i in 0..code.len() {
        if code[i].starts_with("#") {
            panic!("Error: comment at the beginning of line");
        }
        if code[i].ends_with("\n") {
            panic!("No newline symbol at the end of line");
        }
    }

    // finish the program

    add_command(&mut code, "HALT");

    return Ok(code);
}
