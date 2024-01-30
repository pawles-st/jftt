use std::iter::zip;
use num::{BigInt, FromPrimitive, ToPrimitive, Signed};
use crate::ast::*;
use translation_structures::*;

pub mod translation_structures;
pub mod transformation;

// create an entry in the function table for the proc_head
fn malloc_proc(proc_head: &ProcHead, function_table: &mut FunctionTable, code_line_number: usize, mem_addr: u64, location: Location) -> Result<(), TranslationError> {
    let proc_name = proc_head.name.to_owned();

    // check if the procedure name is unique...

    if function_table.contains_key(&proc_name) {
        return Err(TranslationError::RepeatedDeclaration(location, proc_name.clone()));
    }

    // ...if so, add the proc_name and args_decl to the function_table

    function_table.insert(proc_name, ProcedureInfo::new(proc_head.args_decl.clone(), code_line_number, mem_addr));

    Ok(())
}

// create an entry in the symbol table for each variable and array reference
fn malloc_args(mut curr_mem_byte: u64, decls: &ArgumentDeclarations, symbol_table: &mut SymbolTable, location: Location) -> Result<u64, TranslationError> {
    for decl in decls {
        match decl {
            ArgumentDeclaration::Var(pid) => {
                if symbol_table.contains_key(pid) {
                    return Err(TranslationError::RepeatedDeclaration(location, pid.clone()));
                } else {
                    symbol_table.insert(pid.to_owned(), SymbolTableEntry::Var(Variable::new(curr_mem_byte, ValueHeld::Dynamic, true)));
                    curr_mem_byte += 1;
                }
            },
            ArgumentDeclaration::Arr(pid) => {
                if symbol_table.contains_key(pid) {
                    return Err(TranslationError::RepeatedDeclaration(location, pid.clone()));
                } else {
                    symbol_table.insert(pid.to_owned(), SymbolTableEntry::Arr(Array::new(curr_mem_byte, 0, ValueHeld::Dynamic, true)));
                    curr_mem_byte += 1;
                }
            }
        }
    }
    return Ok(curr_mem_byte);
}

// create an entry in the symbol table for each variable and array declaration
fn malloc(mut curr_mem_byte: u64, decls: &Declarations, symbol_table: &mut SymbolTable, location: Location) -> Result<u64, TranslationError> {
    for decl in decls {
        match decl {
            Declaration::Var(pid) => {
                if symbol_table.contains_key(pid) {
                    return Err(TranslationError::RepeatedDeclaration(location, pid.clone()));
                } else {
                    symbol_table.insert(pid.to_owned(), SymbolTableEntry::Var(Variable::new(curr_mem_byte, ValueHeld::Uninitialised, false)));
                    curr_mem_byte += 1;
                }
            },
            Declaration::Arr(pid, len) => {
                if symbol_table.contains_key(pid) {
                    return Err(TranslationError::RepeatedDeclaration(location, pid.clone()));
                } else {
                    symbol_table.insert(pid.to_owned(), SymbolTableEntry::Arr(Array::new(curr_mem_byte, *len, ValueHeld::Uninitialised, false)));
                    curr_mem_byte += *len;
                }
            },
        }
    }
    return Ok(curr_mem_byte);
}

// store the value of the variable held in a given register into memory
// NOTICE: erases the contents of registers A, B
fn store_variable_code(id: &Identifier, register: &Register, symbol_table: &mut SymbolTable, register_states: &mut RegisterStates) -> Vec<String> {
    let mut code = Vec::new();

    println!("storing variable {:?} under register {}", id, register_to_string(&register));

    // load the variable's address

    let mut var_fetch_code = translate_fetch(id, &Register::B, symbol_table, false, None, register_states, (0, 0)).unwrap();
    code.append(&mut var_fetch_code);

    // store the value under the address

    add_command_string(&mut code, "GET ".to_owned() + register_to_string(register));
    add_command(&mut code, "STORE b");

    return code;
}

// store the values of all variables held within registers and change all states to noise
fn reset_register_memory(symbol_table: &mut SymbolTable, register_states: &mut RegisterStates) -> Vec<String> {
    let mut code = Vec::new();

    println!("RESETTING REGISTERS");

    for register in [Register::C, Register::D, Register::E, Register::F, Register::G, Register::H] {
        if let RegisterState::Variable(id) = register_states.registers.get(&register).unwrap() {
            let mut var_store_code = store_variable_code(&id.clone(), &register, symbol_table, register_states);
            code.append(&mut var_store_code);
        }
        register_states.registers.entry(register).and_modify(|state| *state = RegisterState::Noise);
    }
    register_states.registers.entry(Register::A).and_modify(|state| *state = RegisterState::Noise);
    register_states.registers.entry(Register::B).and_modify(|state| *state = RegisterState::Noise);

    println!("RESET COMPLETE: {:?}", register_states);

    return code;
}

// move the value from register A into the register of choice
// NOTICE: this does not update the register state, the caller should do so themselves
fn move_value_code(register: &Register, current_id: Option<&Identifier>, register_states: &mut RegisterStates, symbol_table: &mut SymbolTable) -> Vec<String> {
    if matches!(register, Register::A) {
        return Vec::new();
    } else {
        let mut code = Vec::new();

        // store the currently held variable, if any, in memory

        if let RegisterState::Variable(id) = register_states.registers.get(register).unwrap() {
            if let Some(current) = current_id {
                if current != id {

                    let mut var_store_code = store_variable_code(&id.clone(), register, symbol_table, register_states);
                    add_command(&mut code, "PUT c");
                    code.append(&mut var_store_code);
                    add_command(&mut code, "GET c");

                    register_states.registers.entry(Register::C).and_modify(|e| *e = RegisterState::Noise);

                }
            } else {

                let mut var_store_code = store_variable_code(&id.clone(), register, symbol_table, register_states);
                add_command(&mut code, "PUT c");
                code.append(&mut var_store_code);
                add_command(&mut code, "GET c");

                register_states.registers.entry(Register::C).and_modify(|e| *e = RegisterState::Noise);

            }
        }

        // copy the value of register A into the register of choice

        add_command_string(&mut code, "PUT ".to_owned() + register_to_string(register));

        return code;
    }
}

// TODO: create the new value based on the old one in more cases with shifts and DECs
// create the specified Num (u64) value and store it in the register of choice
fn translate_load_const(value: Num, register: &Register, register_states: &mut RegisterStates, symbol_table: &mut SymbolTable) -> Vec<String> {
    let mut code = Vec::new();

    let modified_register_state = register_states.registers.get_mut(register).unwrap();

    let register_str = register_to_string(register);

    // if the current value of the register is "close", increment/decrement it to get the result

    if let RegisterState::Constant(curr) = modified_register_state {
        let difference = BigInt::from_u64(value).unwrap() - curr.clone();
        *modified_register_state = RegisterState::Constant(BigInt::from_u64(value).unwrap());

        if difference == BigInt::from_i64(0).unwrap() {
            return Vec::new();
        } else if difference > BigInt::from_i64(0).unwrap() && difference <= BigInt::from_i64(3).unwrap() {
            return vec![String::from("INC ".to_owned() + register_str)].iter().cycle().take(difference.abs().to_usize().unwrap()).cloned().collect();
        } else if difference < BigInt::from_i64(0).unwrap() && difference >= BigInt::from_i64(-3).unwrap() {
            return vec![String::from("DEC ".to_owned() + register_str)].iter().cycle().take(difference.abs().to_usize().unwrap()).cloned().collect();
        } // else: continue on
    }

    // store the currently held variable, if any, in memory

    if let RegisterState::Variable(id) = register_states.registers.get(register).unwrap() {
        let mut var_store_code = store_variable_code(&id.clone(), register, symbol_table, register_states);
        code.append(&mut var_store_code);
    }

    // modify the register's state

    register_states.registers.entry(register.clone()).and_modify(|state| *state = RegisterState::Constant(BigInt::from_u64(value).unwrap()));

    // reset the chosen register

    add_command_string(&mut code, "RST ".to_owned() + register_str);

    if value == 0 {
        return code;
    }

    // start with the value of msb as 1

    let set_msb_code = "INC ".to_owned() + register_str;
    code.push(set_msb_code);

    // create the binary representation of the number

    let value_binary: Vec<char> = format!("{:b}", value).chars().collect();

    for bit in 1..value_binary.len() {
        let shift_code = "SHL ".to_owned() + register_str;
        code.push(shift_code);
        if value_binary[bit] == '1' {
            let set_bit_code = "INC ".to_owned() + register_str;
            code.push(set_bit_code);
        }
    }

    return code;
}

// fetch the address of a Pidentifier into the register of choice
// NOTICE: erases the contents of registers A and B
fn translate_fetch_pid(varname: &Pidentifier, register: &Register, symbol_table: &mut SymbolTable, check_initialisation: bool, update_value: Option<ValueHeld>, register_states: &mut RegisterStates, location: Location) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    // check if the varname exists in the symbol table...

    if let Some(entry) = symbol_table.get_mut(varname) {

        // ...and whether it's not an array

        if let SymbolTableEntry::Var(var) = entry {

            // check initialisation if needed

            /*
            if check_initialisation {
                if matches!(var.value, ValueHeld::Uninitialised) {
                    return Err(TranslationError::UninitialisedVariable(location, varname.clone()));
                }
            }
            */

            // update the value in the symbol table if required

            if let Some(valtype) = update_value {
                var.value = valtype;
            }

            // next, check whether the variable holds a reference...

            if var.is_ref {

                // ...if so, load the reference's address into register B

                let mut ref_address_code = translate_load_const(var.memloc, &Register::B, register_states, symbol_table);
                code.append(&mut ref_address_code);

                let comment = varname.to_owned() + " IS ref; indirectly fetching address into register " + register_to_string(register);
                add_comment(&mut code, &comment);

                // next, load the value stored under the reference's address
                // into register A - that is the original variables's address

                add_command(&mut code, "LOAD b");

                // if the resulting address is to be stored in a register other than A, move it

                code.append(&mut move_value_code(register, None, register_states, symbol_table));
            } else {

                // ..otherwise, load the address directly into the specified register

                let mut pid_address_code = translate_load_const(var.memloc, register, register_states, symbol_table);
                code.append(&mut pid_address_code);

                let comment = varname.to_owned() + " is NOT ref; directly fetching address into register " + register_to_string(register);
                add_comment(&mut code, &comment);
            }
        } else {
            return Err(TranslationError::NoArrayIndex(location, varname.clone()));
        }
    } else {
        return Err(TranslationError::NoSuchVariable(location, varname.clone()));
    }
    return Ok(code);
}

// fetch the address of a specified array element into the register of choice
// NOTICE: erases the contents of registers A and B
// TODO: array bound checking
fn translate_fetch_arrnum(arrname: &Pidentifier, idx: Num, register: &Register, symbol_table: &mut SymbolTable, check_initialisation: bool, update_value: Option<ValueHeld>, register_states: &mut RegisterStates, location: Location) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();
    
    // check if the arrname exists in the symbol table...

    if let Some(entry) = symbol_table.get_mut(arrname) {

        // ...and assert it is an array

        if let SymbolTableEntry::Arr(arr) = entry {

            // check initialisation if needed

            /*
            if check_initialisation {
                if matches!(arr.value, ValueHeld::Uninitialised) {
                    return Err(TranslationError::UninitialisedVariable(location, arrname.clone()));
                }
            }
            */
            
            // update the value in the symbol table if required

            if let Some(valtype) = update_value {
                arr.value = valtype;
            }

            // next, check whether the variable holds a reference...

            if arr.is_ref {

                // if so, load the reference's address into register B

                let mut ref_address_code = translate_load_const(arr.memloc, &Register::B, register_states, symbol_table);
                code.append(&mut ref_address_code);

                let comment = arrname.to_owned() + " IS array ref; indirectly fetching address into register " + register_to_string(register);
                add_comment(&mut code, &comment);

                // next, load the value stored under the reference's address
                // into register A - that is the array's beginning address

                add_command(&mut code, "LOAD b");

                // load the array index into register B

                let mut idx_load_code = translate_load_const(idx, &Register::B, register_states, symbol_table);
                code.append(&mut idx_load_code);

                // add the two together to get the final address

                add_command(&mut code, "ADD b");

                // if the resulting address is to be stored in a register other than A, move it

                code.append(&mut move_value_code(register, None, register_states, symbol_table));
            } else {
                
                // ..otherwise, load the address directly into the specified register

                let mut arrnum_address_code = translate_load_const(arr.memloc + idx, register, register_states, symbol_table);
                code.append(&mut arrnum_address_code);

                let comment = arrname.to_owned() + " is NOT array ref; directly fetching address into register " + register_to_string(register);
                add_comment(&mut code, &comment);
            }
        } else {
            return Err(TranslationError::NotAnArray(location, arrname.clone()));
        }
    } else {
        return Err(TranslationError::NoSuchVariable(location, arrname.clone()));
    }
    return Ok(code);
}

// fetch the address of an array entry with index equal to the value
// of the indexing variable and store it in the register of choice
// NOTICE: erases the contents of registers A, B and C
fn translate_fetch_arrpid(arrname: &Pidentifier, idx_varname: &Pidentifier, register: &Register, symbol_table: &mut SymbolTable, register_states: &mut RegisterStates, location: Location) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();
    
    // fetch the address of the indexing variable into register B...

    let mut fetch_idx_var_code = translate_fetch_pid(idx_varname, &Register::B, symbol_table, true, None, register_states, location)?;
    code.append(&mut fetch_idx_var_code);
    
    let comment = "fetching ".to_owned() + arrname + "[" + idx_varname + "]'s address into register " + register_to_string(register);
    add_comment(&mut code, &comment);

    // ...then load its value into register A...

    add_command(&mut code, "LOAD b");

    // ...and temporarily store it in register C

    code.append(&mut move_value_code(&Register::C, None, register_states, symbol_table));

    // next, load the array address into register A
   
    let mut fetch_arr_code = translate_fetch_arrnum(arrname, 0, &Register::A, symbol_table, false, None, register_states, location)?;
    code.append(&mut fetch_arr_code);

    // finally, add the address of the array in register A to the value of the
    // indexing variable in register C to get the final address

    let mut offset_code = Vec::new();
    add_command(&mut offset_code, "ADD c");

    let comment = "calculating address of ".to_owned() + arrname + "[" + idx_varname + "]";
    add_comment(&mut offset_code, &comment);

    code.append(&mut offset_code);

    // if the resulting address is to be stored in a register other than A, move it

    code.append(&mut move_value_code(register, None, register_states, symbol_table));

    return Ok(code);
}

// fetch the address of the specified Identifier into the register of choice
// NOTICE: erases the contents of registers A, B and E
fn translate_fetch(id: &Identifier, register: &Register, symbol_table: &mut SymbolTable, check_initialisation: bool, update_value: Option<ValueHeld>, register_states: &mut RegisterStates, location: Location) -> Result<Vec<String>, TranslationError> {

    // execute the appropriate fetch code based on the Identifier type

    match id {
        Identifier::Pid(varname) => 
            return translate_fetch_pid(varname, register, symbol_table, check_initialisation, update_value, register_states, location),
        Identifier::ArrNum(arrname, idx) =>
            return translate_fetch_arrnum(arrname, *idx, register, symbol_table, check_initialisation, update_value, register_states, location),
        Identifier::ArrPid(arrname, idx_varname) =>
            return translate_fetch_arrpid(arrname, idx_varname, register, symbol_table, register_states, location),
    }
}

// fetch the specified Value into the register of choice
// NOTICE: erases the contents of registers A, B and C
fn translate_val(value: &Value, register: &Register, symbol_table: &mut SymbolTable, register_states: &mut RegisterStates, location: Location) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    match value {
        Value::Number(num) => {

            // generate the const value in the chosen register

            code.append(&mut translate_load_const(*num, register, register_states, symbol_table));

            let comment = "generating constant ".to_owned() + &num.to_string() + " into register " + &register_to_string(register);
            add_comment(&mut code, &comment);

        },
        Value::Id(id) => {
            if let Some(val_register) = register_states.scan(id) {

                // if the variable is already loaded, move it to the result register,

                if *register != val_register {
                    add_command_string(&mut code, "GET ".to_owned() + register_to_string(&val_register));
                    code.append(&mut move_value_code(register, Some(id), register_states, symbol_table));
                    
                }
            } else {

                // fetch the address of the Identifier into register B...

                let mut fetch_id_code = translate_fetch(id, &Register::B, symbol_table, true, None, register_states, location)?;
                code.append(&mut fetch_id_code);

                // ...and load its value into the specified register

                let mut load_code = Vec::new();
                add_command(&mut load_code, "LOAD b");

                let comment = "loading ".to_owned() + &format!("{:?}", id) + "'s value into register " + &register_to_string(register);
                add_comment(&mut load_code, &comment);

                code.append(&mut load_code);

                code.append(&mut move_value_code(register, None, register_states, symbol_table));
                
                // update the register's state

                if (matches!(id, Identifier::Pid{..}) || matches!(id, Identifier::ArrNum{..})) && register != &Register::A && register != &Register::B {
                    println!("saving");
                    register_states.registers.entry(register.clone()).and_modify(|state| *state = RegisterState::Variable(id.clone()));
                } else {
                    register_states.registers.entry(register.clone()).and_modify(|state| *state = RegisterState::Noise);
                }
            }
        },
    }
    
    return Ok(code);
}

// perform an add Expression for lhs and rhs Values and store the result in the register of choice
// NOTICE: erases the contents of registers A, B, C, aux
fn translate_add_expr(id: &Identifier, lhs: &Value, rhs: &Value, symbol_table: &mut SymbolTable, register_states: &mut RegisterStates, location: Location) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    // load the rhs value

    // check if a register is already holding the value

    let rhs_register;

    if let Value::Id(lhs_id) = rhs {
        if let Some(register) = register_states.scan(lhs_id) {

            // if the variable is already loaded, do nothing,

            rhs_register = register;

        } else {
            
            // if any of the conditions fail, load its value into the next register
            
            rhs_register = register_states.get_next();
            let mut rhs_code = translate_val(rhs, &rhs_register, symbol_table, register_states, location)?;
            code.append(&mut rhs_code);

        }
    } else {
        
        // if any of the conditions fail, load its value into the next register
        
        rhs_register = register_states.get_next();
        let mut rhs_code = translate_val(rhs, &rhs_register, symbol_table, register_states, location)?;
        code.append(&mut rhs_code);

    }
    
    // load the lhs value

    // check if a register is already holding the value
    
    let lhs_register;

    if let Value::Id(rhs_id) = lhs {
        if let Some(register) = register_states.scan(rhs_id) {

            // if the variable is already loaded, do nothing

            lhs_register = register;

        } else {
            
            // if any of the conditions fail, load its value into the next register
            
            lhs_register = register_states.get_next();
            let mut lhs_code = translate_val(lhs, &lhs_register, symbol_table, register_states, location)?;
            code.append(&mut lhs_code);

        }
    } else {
        
        // if any of the conditions fail, load its value into the next register
        
        lhs_register = register_states.get_next();
        let mut lhs_code = translate_val(lhs, &lhs_register, symbol_table, register_states, location)?;
        code.append(&mut lhs_code);

    }

    // store the Expression value in the next register

    let result_register;

    if let Some(register) = register_states.scan(id) {
        result_register = register;
    } else {
        result_register = register_states.get_next();
    }

    let comment = format!("{:?}", lhs) + " + " + &format!("{:?}", rhs);
    add_comment(&mut code, &comment);

    // add the values 

    let mut addition_code = Vec::new();
    add_command_string(&mut addition_code, "GET ".to_owned() + register_to_string(&lhs_register));
    add_command_string(&mut addition_code, "ADD ".to_owned() + register_to_string(&rhs_register));

    let comment = "performing addition; storing in register ".to_owned() + &register_to_string(&result_register);
    add_comment(&mut addition_code, &comment);

    code.append(&mut addition_code);
    
    // move the result into the register of choice
    
    code.append(&mut move_value_code(&result_register, Some(id), register_states, symbol_table));
    
    register_states.registers.entry(result_register).and_modify(|e| *e = RegisterState::Variable(id.clone()));
    register_states.registers.entry(Register::A).and_modify(|e| *e = RegisterState::Noise);

    return Ok(code);
}

// perform the sub Expression for lhs and rhs Values and store the result in the register of choice
// NOTICE: erases the contents of registers A, B, C, aux
fn translate_sub_expr(id: &Identifier, lhs: &Value, rhs: &Value, symbol_table: &mut SymbolTable, register_states: &mut RegisterStates, location: Location) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();
    
    // load the rhs value

    // check if a register is already holding the value

    let rhs_register;

    if let Value::Id(id) = rhs {
        if let Some(register) = register_states.scan(id) {

            // if the variable is already loaded, do nothing,

            rhs_register = register;

        } else {
            
            // if any of the conditions fail, load its value into the next register
            
            rhs_register = register_states.get_next();
            let mut rhs_code = translate_val(rhs, &rhs_register, symbol_table, register_states, location)?;
            code.append(&mut rhs_code);

        }
    } else {
        
        // if any of the conditions fail, load its value into the next register
        
        rhs_register = register_states.get_next();
        let mut rhs_code = translate_val(rhs, &rhs_register, symbol_table, register_states, location)?;
        code.append(&mut rhs_code);

    }
    
    // load the lhs value

    // check if a register is already holding the value
    
    let lhs_register;

    if let Value::Id(id) = lhs {
        if let Some(register) = register_states.scan(id) {

            // if the variable is already loaded, do nothing

            lhs_register = register;

        } else {
            
            // if any of the conditions fail, load its value into the next register
            
            lhs_register = register_states.get_next();
            let mut lhs_code = translate_val(lhs, &lhs_register, symbol_table, register_states, location)?;
            code.append(&mut lhs_code);

        }
    } else {
        
        // if any of the conditions fail, load its value into the next register
        
        lhs_register = register_states.get_next();
        let mut lhs_code = translate_val(lhs, &lhs_register, symbol_table, register_states, location)?;
        code.append(&mut lhs_code);

    }
   
    // store the Expression value in the next register

    let result_register;

    if let Some(register) = register_states.scan(id) {
        result_register = register;
    } else {
        result_register = register_states.get_next();
    }

    let comment = format!("{:?}", lhs) + " - " + &format!("{:?}", rhs);
    add_comment(&mut code, &comment);

    // subtract the values

    let mut subtraction_code = Vec::new();
    add_command_string(&mut subtraction_code, "GET ".to_owned() + register_to_string(&lhs_register));
    add_command_string(&mut subtraction_code, "SUB ".to_owned() + register_to_string(&rhs_register));

    let comment = "performing subtraction; storing in register ".to_owned() + &register_to_string(&result_register);
    add_comment(&mut subtraction_code, &comment);

    code.append(&mut subtraction_code);

    // move the result into the register of choice

    code.append(&mut move_value_code(&result_register, Some(id), register_states, symbol_table));
    
    register_states.registers.entry(result_register).and_modify(|e| *e = RegisterState::Variable(id.clone()));
    register_states.registers.entry(Register::A).and_modify(|e| *e = RegisterState::Noise);

    return Ok(code);
}

fn multiply_code(curr_line: usize, lhs_register: &Register, rhs_register: &Register, result_register: &Register) -> Vec<String> {
    let mut code = Vec::new();


    add_command_string(&mut code, "RST ".to_owned() + register_to_string(result_register));

    // fetch the still-left rhs
    // if it's equal to zero, stop

    add_command_string(&mut code, "GET ".to_owned() + register_to_string(rhs_register)); // label: mul_loop_line
    let end_loop_line = (curr_line + 13).to_string();
    add_command_string(&mut code, "JZERO ".to_owned() + &end_loop_line);

    // see if lsb of still-left rhs is 1...

    add_command_string(&mut code, "SHR ".to_owned() + register_to_string(rhs_register));
    add_command_string(&mut code, "SHL ".to_owned() + register_to_string(rhs_register));
    add_command_string(&mut code, "SUB ".to_owned() + register_to_string(rhs_register));

    // ...if not, don't add anything

    let after_add_line = (curr_line + 10).to_string();
    add_command_string(&mut code, "JZERO ".to_owned() + &after_add_line);

    // ...if it is a 1, add the current lhs shift to the result

    add_command_string(&mut code, "GET ".to_owned() + register_to_string(result_register));
    add_command_string(&mut code, "ADD ".to_owned() + register_to_string(lhs_register));
    add_command_string(&mut code, "PUT ".to_owned() + register_to_string(result_register));

    // shift the lhs to the left, rhs to the right

    add_command_string(&mut code, "SHL ".to_owned() + register_to_string(lhs_register)); // label: after_add_line
    add_command_string(&mut code, "SHR ".to_owned() + register_to_string(rhs_register));

    // repeat until rhs is 0

    let mul_loop_line = (curr_line + 1).to_string();
    add_command_string(&mut code, "JUMP ".to_owned() + &mul_loop_line);
    // label: end_loop_line

    return code
}

// perform the mul Expression for lhs and rhs Values and store the result in the register of choice
// TODO: optimise multiplication by a constant
// NOTICE: erases the contents of registers A, B, aux1, aux2, aux3
fn translate_mul_expr(id: &Identifier, lhs: &Value, rhs: &Value, symbol_table: &mut SymbolTable, mut curr_line: usize, register_states: &mut RegisterStates, location: Location) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    // load the rhs value

    // check if a register is already holding the value

    let rhs_register;

    if let Value::Id(id) = rhs {
        if let Some(register) = register_states.scan(id) {

            // if the variable is already loaded, do nothing,

            rhs_register = register;

        } else {

            // if any of the conditions fail, load its value into the next register

            rhs_register = register_states.get_next();
            let mut rhs_code = translate_val(rhs, &rhs_register, symbol_table, register_states, location)?;
            code.append(&mut rhs_code);

        }
    } else {

        // if any of the conditions fail, load its value into the next register

        rhs_register = register_states.get_next();
        let mut rhs_code = translate_val(rhs, &rhs_register, symbol_table, register_states, location)?;
        code.append(&mut rhs_code);

    }

    // load the lhs value

    // check if a register is already holding the value
    
    let lhs_register;

    if let Value::Id(id) = lhs {
        if let Some(register) = register_states.scan(id) {

            // if the variable is already loaded, do nothing

            lhs_register = register;

        } else {
            
            // if any of the conditions fail, load its value into the next register
            
            lhs_register = register_states.get_next();
            let mut lhs_code = translate_val(lhs, &lhs_register, symbol_table, register_states, location)?;
            code.append(&mut lhs_code);

        }
    } else {
        
        // if any of the conditions fail, load its value into the next register
        
        lhs_register = register_states.get_next();
        let mut lhs_code = translate_val(lhs, &lhs_register, symbol_table, register_states, location)?;
        code.append(&mut lhs_code);

    }

    // store the Expression value in the next register

    let result_register;

    if let Some(register) = register_states.scan(id) {
        result_register = register;
    } else {
        result_register = register_states.get_next();
    }

    let comment = format!("{:?}", lhs) + " * " + &format!("{:?}", rhs);
    add_comment(&mut code, &comment);
    
    // store the variable held by the result register, if any
    
    if let RegisterState::Variable(current_id) = register_states.registers.get(&result_register).unwrap() {
        if id != current_id {
            let mut var_store_code = store_variable_code(&current_id.clone(), &result_register, symbol_table, register_states);
            code.append(&mut var_store_code);
        }
    }

    // copy the lhs into first auxiliary register

    let aux1_register = register_states.get_next();
    add_command_string(&mut code, "GET ".to_owned() + register_to_string(&lhs_register));
    code.append(&mut move_value_code(&aux1_register, None, register_states, symbol_table));
    
    // copy the rhs into second auxiliary register

    let aux2_register = register_states.get_next();
    add_command_string(&mut code, "GET ".to_owned() + register_to_string(&rhs_register));
    code.append(&mut move_value_code(&aux2_register, None, register_states, symbol_table));

    // multiply the values
    
    curr_line += code.len();

    let mut multiplication_code = multiply_code(curr_line, &aux1_register, &aux2_register, &result_register);
    code.append(&mut multiplication_code);
    
    register_states.registers.entry(result_register).and_modify(|e| *e = RegisterState::Variable(id.clone()));
    register_states.registers.entry(aux1_register.clone()).and_modify(|e| *e = RegisterState::Noise);
    register_states.registers.entry(aux2_register.clone()).and_modify(|e| *e = RegisterState::Noise);
    register_states.registers.entry(Register::A).and_modify(|e| *e = RegisterState::Noise);

    return Ok(code);
}

fn divide_code(curr_line: usize, lhs_register: &Register, rhs_register: &Register, mut result_register: &Register, division_type: &DivisionType) -> Vec<String> {
    let mut code = Vec::new();

    let mut rem_register = &Register::B;

    if matches!(division_type, DivisionType::Modulo) {

        // swap the quotient and modulo registers in case of modulo operation

        let temp_reg = rem_register;
        rem_register = result_register;
        result_register = temp_reg;
    }

    // register B will hold the quotient, register E the remainder
    // reset the quotient and remainder register

    add_command_string(&mut code, "RST ".to_owned() + register_to_string(result_register));
    add_command_string(&mut code, "RST ".to_owned() + register_to_string(rem_register));

    // if divisor is 0, stop

    add_command_string(&mut code, "GET ".to_owned() + register_to_string(rhs_register));
    add_command_string(&mut code, "JZERO ".to_owned() + &(curr_line + 26).to_string());

    // copy dividend into the remainder register

    add_command_string(&mut code, "GET ".to_owned() + register_to_string(lhs_register));
    add_command_string(&mut code, "PUT ".to_owned() + register_to_string(rem_register));

    // copy original value of divisor into register C (dividend is no longer needed)

    add_command_string(&mut code, "GET ".to_owned() + register_to_string(rhs_register));
    add_command_string(&mut code, "PUT ".to_owned() + register_to_string(lhs_register));

    // shift divisor left as long as it's smaller than still-left dividend

    add_command_string(&mut code, "SHL ".to_owned() + register_to_string(rhs_register)); // label: align_divisor
    add_command_string(&mut code, "GET ".to_owned() + register_to_string(rhs_register));
    add_command_string(&mut code, "SUB ".to_owned() + register_to_string(rem_register));
    add_command_string(&mut code, "JZERO ".to_owned() + &(curr_line + 8).to_string());
    add_command_string(&mut code, "SHR ".to_owned() + register_to_string(rhs_register));
    //add_command_string(&mut code, "JUMP {divide}");

    // perform iterative divison by subtraction of decreasing multiples of divisor
    // finish when the value in register D reaches the original value of the divisor

    // shift the quotient to the left

    add_command_string(&mut code, "SHL ".to_owned() + register_to_string(result_register)); // label: divide

    // check if dividend >= divisor...

    add_command_string(&mut code, "GET ".to_owned() + register_to_string(rhs_register));
    add_command_string(&mut code, "SUB ".to_owned() + register_to_string(rem_register));

    // ...if not, jump to next iteration

    add_command_string(&mut code, "JPOS ".to_owned() + &(curr_line + 21).to_string());

    // ...otherwise, subtract from the still-left dividend and increment the quotient by one

    add_command_string(&mut code, "GET ".to_owned() + register_to_string(rem_register));
    add_command_string(&mut code, "SUB ".to_owned() + register_to_string(rhs_register));
    add_command_string(&mut code, "PUT ".to_owned() + register_to_string(rem_register));
    add_command_string(&mut code, "INC ".to_owned() + register_to_string(result_register));

    // shift the divisor to the right and check if the new value is smaller than the original
    // (divided by all multplies of the divisor)
    // if so, stop division; otherwise, loop and continue

    add_command_string(&mut code, "SHR ".to_owned() + register_to_string(rhs_register)); // check_end
    add_command_string(&mut code, "GET ".to_owned() + register_to_string(lhs_register));
    add_command_string(&mut code, "SUB ".to_owned() + register_to_string(rhs_register));
    add_command_string(&mut code, "JZERO ".to_owned() + &(curr_line + 13).to_string());
    // label: finish

    return code;
}

fn translate_div_expr(id: &Identifier, lhs: &Value, rhs: &Value, symbol_table: &mut SymbolTable, mut curr_line: usize, register_states: &mut RegisterStates, location: Location) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();
    
    // load the rhs value

    // check if a register is already holding the value

    let rhs_register;

    if let Value::Id(id) = rhs {
        if let Some(register) = register_states.scan(id) {

            // if the variable is already loaded, do nothing,

            rhs_register = register;

        } else {

            // if any of the conditions fail, load its value into the next register

            rhs_register = register_states.get_next();
            let mut rhs_code = translate_val(rhs, &rhs_register, symbol_table, register_states, location)?;
            code.append(&mut rhs_code);

        }
    } else {

        // if any of the conditions fail, load its value into the next register

        rhs_register = register_states.get_next();
        let mut rhs_code = translate_val(rhs, &rhs_register, symbol_table, register_states, location)?;
        code.append(&mut rhs_code);

    }

    // load the lhs value

    // check if a register is already holding the value

    let lhs_register;

    if let Value::Id(id) = lhs {
        if let Some(register) = register_states.scan(id) {

            // if the variable is already loaded, do nothing

            lhs_register = register;

        } else {
            
            // if any of the conditions fail, load its value into the next register
            
            lhs_register = register_states.get_next();
            let mut lhs_code = translate_val(lhs, &lhs_register, symbol_table, register_states, location)?;
            code.append(&mut lhs_code);

        }
    } else {
        
        // if any of the conditions fail, load its value into the next register
        
        lhs_register = register_states.get_next();
        let mut lhs_code = translate_val(lhs, &lhs_register, symbol_table, register_states, location)?;
        code.append(&mut lhs_code);

    }

    // store the Expression value in the next register

    let result_register;

    if let Some(register) = register_states.scan(id) {
        result_register = register;
    } else {
        result_register = register_states.get_next();
    }

    let comment = format!("{:?}", lhs) + " / " + &format!("{:?}", rhs);
    add_comment(&mut code, &comment);
    
    // store the variable held by the result register, if any
    
    if let RegisterState::Variable(current_id) = register_states.registers.get(&result_register).unwrap() {
        if id != current_id {
            let mut var_store_code = store_variable_code(&current_id.clone(), &result_register, symbol_table, register_states);
            code.append(&mut var_store_code);
        }
    }

    // copy the lhs into first auxiliary register

    let aux1_register = register_states.get_next();
    add_command_string(&mut code, "GET ".to_owned() + register_to_string(&lhs_register));
    code.append(&mut move_value_code(&aux1_register, None, register_states, symbol_table));
    
    // copy the rhs into second auxiliary register

    let aux2_register = register_states.get_next();
    add_command_string(&mut code, "GET ".to_owned() + register_to_string(&rhs_register));
    code.append(&mut move_value_code(&aux2_register, None, register_states, symbol_table));

    // divide the values

    curr_line += code.len();
    
    let mut division_code = divide_code(curr_line, &aux1_register, &aux2_register, &result_register, &DivisionType::Division);
    code.append(&mut division_code);
    
    register_states.registers.entry(result_register).and_modify(|e| *e = RegisterState::Variable(id.clone()));
    register_states.registers.entry(aux1_register.clone()).and_modify(|e| *e = RegisterState::Noise);
    register_states.registers.entry(aux2_register.clone()).and_modify(|e| *e = RegisterState::Noise);
    register_states.registers.entry(Register::A).and_modify(|e| *e = RegisterState::Noise);
    register_states.registers.entry(Register::B).and_modify(|e| *e = RegisterState::Noise);

    return Ok(code);
}

fn translate_mod_expr(id: &Identifier, lhs: &Value, rhs: &Value, symbol_table: &mut SymbolTable, mut curr_line: usize, register_states: &mut RegisterStates, location: Location) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();
    
    // load the rhs value

    // check if a register is already holding the value

    let rhs_register;

    if let Value::Id(id) = rhs {
        if let Some(register) = register_states.scan(id) {

            // if the variable is already loaded, do nothing,

            rhs_register = register;

        } else {

            // if any of the conditions fail, load its value into the next register

            rhs_register = register_states.get_next();
            let mut rhs_code = translate_val(rhs, &rhs_register, symbol_table, register_states, location)?;
            code.append(&mut rhs_code);

        }
    } else {

        // if any of the conditions fail, load its value into the next register

        rhs_register = register_states.get_next();
        let mut rhs_code = translate_val(rhs, &rhs_register, symbol_table, register_states, location)?;
        code.append(&mut rhs_code);

    }

    // load the lhs value

    // check if a register is already holding the value

    let lhs_register;

    if let Value::Id(id) = lhs {
        if let Some(register) = register_states.scan(id) {

            // if the variable is already loaded, do nothing

            lhs_register = register;

        } else {
            
            // if any of the conditions fail, load its value into register A
            
            lhs_register = register_states.get_next();
            let mut lhs_code = translate_val(lhs, &lhs_register, symbol_table, register_states, location)?;
            code.append(&mut lhs_code);

        }
    } else {
        
        // if any of the conditions fail, load its value into register A
        
        lhs_register = register_states.get_next();
        let mut lhs_code = translate_val(lhs, &lhs_register, symbol_table, register_states, location)?;
        code.append(&mut lhs_code);

    }

    // store the Expression value in the next register

    let result_register;

    if let Some(register) = register_states.scan(id) {
        result_register = register;
    } else {
        result_register = register_states.get_next();
    }
    
    let comment = format!("{:?}", lhs) + " % " + &format!("{:?}", rhs);
    add_comment(&mut code, &comment);
    
    // store the variable held by the result register, if any
    
    if let RegisterState::Variable(current_id) = register_states.registers.get(&result_register).unwrap() {
        if id != current_id {
            let mut var_store_code = store_variable_code(&current_id.clone(), &result_register, symbol_table, register_states);
            code.append(&mut var_store_code);
        }
    }

    // copy the lhs into first auxiliary register

    let aux1_register = register_states.get_next();
    add_command_string(&mut code, "GET ".to_owned() + register_to_string(&lhs_register));
    code.append(&mut move_value_code(&aux1_register, None, register_states, symbol_table));
    
    // copy the rhs into second auxiliary register

    let aux2_register = register_states.get_next();
    add_command_string(&mut code, "GET ".to_owned() + register_to_string(&rhs_register));
    code.append(&mut move_value_code(&aux2_register, None, register_states, symbol_table));

    // divide the values
    
    curr_line += code.len();

    let mut division_code = divide_code(curr_line, &aux1_register, &aux2_register, &result_register, &DivisionType::Modulo);
    code.append(&mut division_code);
 
    register_states.registers.entry(result_register).and_modify(|e| *e = RegisterState::Variable(id.clone()));
    register_states.registers.entry(aux1_register.clone()).and_modify(|e| *e = RegisterState::Noise);
    register_states.registers.entry(aux2_register.clone()).and_modify(|e| *e = RegisterState::Noise);
    register_states.registers.entry(Register::A).and_modify(|e| *e = RegisterState::Noise);
    register_states.registers.entry(Register::B).and_modify(|e| *e = RegisterState::Noise);

    return Ok(code);
}

// calculate the value of the specified Expression and store the result in the register of choice
// NOTICE: erases the contents of registers A, B, C, D and E
fn translate_expr(id: &Identifier, expr: &Expression, symbol_table: &mut SymbolTable, curr_line: usize, register_states: &mut RegisterStates, location: Location) -> Result<Vec<String>, TranslationError> {
    println!("{:?} = {:?}, {:?}", id, expr, register_states);
    match expr {
        Expression::Val(value) => {

            // store the Expression value in the next register

            let result_register;

            if let Some(register) = register_states.scan(id) {
                result_register = register;
            } else {
                result_register = register_states.get_next();
            }

            let val_code = translate_val(value, &result_register, symbol_table, register_states, location);

            register_states.registers.entry(result_register).and_modify(|state| *state = RegisterState::Variable(id.clone()));
            return val_code;
        }
        Expression::Add(lhs, rhs) =>
            return translate_add_expr(id, lhs, rhs, symbol_table, register_states, location),
        Expression::Sub(lhs, rhs) =>
            return translate_sub_expr(id, lhs, rhs, symbol_table, register_states, location),
        Expression::Mul(lhs, rhs) =>
            return translate_mul_expr(id, lhs, rhs, symbol_table, curr_line, register_states, location),
        Expression::Div(lhs, rhs) =>
            return translate_div_expr(id, lhs, rhs, symbol_table, curr_line, register_states, location),
        Expression::Mod(lhs, rhs) =>
            return translate_mod_expr(id, lhs, rhs, symbol_table, curr_line, register_states, location),
    }
}

// store the value of the rhs Expression at the address of the lhs Identifier
fn translate_assignment(id: &Identifier, expr: &Expression, symbol_table: &mut SymbolTable, curr_line: usize, register_states: &mut RegisterStates, location: Location) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    let mut expr_code = translate_expr(id, expr, symbol_table, curr_line, register_states, location)?;
    code.append(&mut expr_code);

    return Ok(code);
}

// return from the procedure to the caller
fn translate_return(symbol_table: &mut SymbolTable, register_states: &mut RegisterStates) -> Vec<String> {
    let mut code = Vec::new();

    // assert the return location object has been stored in the symbol table...

    if let Some(ret) = symbol_table.get(".return") {

        // ...and that it is of a correct type

        if let SymbolTableEntry::Ret(return_location) = ret {
            let ret_mem_loc = return_location.memloc.clone();

            // store all variables currently kept in registers

            let variable_registers = [Register::C, Register::D, Register::E, Register::F, Register::G, Register::H];
            for register in variable_registers {
                if let RegisterState::Variable(id) = register_states.registers.get(&register).unwrap() {
                    let mut var_store_code = store_variable_code(&id.clone(), &register, symbol_table, register_states);
                    code.append(&mut var_store_code);

                    register_states.registers.entry(register).and_modify(|e| *e = RegisterState::Noise);
                }
            }
            
            // load the return address

            let mut ret_addr_code = translate_load_const(ret_mem_loc, &Register::B, register_states, symbol_table);
            code.append(&mut ret_addr_code);
            add_command(&mut code, "LOAD b");

            let comment = "return to the caller";
            add_comment(&mut code, comment);

            // ...and jump to the line number equal to this value

            add_command(&mut code, "JUMPR a");
            
            register_states.registers.entry(Register::B).and_modify(|e| *e = RegisterState::Noise);
        } else {
            panic!("Expected return address object in the symbol table, found {:?}", ret);
        }
    } else {
        panic!("Expected a return address object in the symbol table, but none was found");
    }
    return code;
}


fn translate_store_var_reference(arg_memloc: u64, is_ref: bool, store_memloc: u64, symbol_table: &mut SymbolTable, register_states: &mut RegisterStates) -> Vec<String> {
    let mut code = Vec::new();

    if is_ref {

        // if the variable is a reference, first load the reference's address into register B...

        let mut fetch_ref_code = translate_load_const(arg_memloc, &Register::B, register_states, symbol_table);
        code.append(&mut fetch_ref_code);

        // ...and then fetch the value stored under it (original var's address) into register A

        add_command(&mut code, "LOAD b");

        // load the store memory location into register B

        let mut fetch_store_code = translate_load_const(store_memloc, &Register::B, register_states, symbol_table);
        code.append(&mut fetch_store_code);

        // store the original variable's address

        add_command(&mut code, "STORE b");
    } else {

        // if the variable isn't a reference, load the variable's address into register A

        let mut fetch_var_code = translate_load_const(arg_memloc, &Register::A, register_states, symbol_table);
        code.append(&mut fetch_var_code);

        // load the store memory location into register B

        let mut fetch_store_code = translate_load_const(store_memloc, &Register::B, register_states, symbol_table);
        code.append(&mut fetch_store_code);

        // store the variable's address

        add_command(&mut code, "STORE b");
    }
    return code;
}

// call a procedure with given arguments
fn translate_proc_call(name: &Pidentifier, args: &Arguments, symbol_table: &mut SymbolTable, function_table: &FunctionTable, curr_proc: Option<&Pidentifier>, register_states: &mut RegisterStates, location: Location) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    // check for a recursive call

    if let Some(curr_proc_name) = curr_proc {
        if name == curr_proc_name {
            return Err(TranslationError::RecurrenceNotAllowed(location, curr_proc_name.clone()));
        }
    }

    // fetch the destination procedure information from the function table

    if let Some(proc_info) = function_table.get(name) {

        // check if the number of arguments matches the number
        // of parameters of the destination procedure

        if args.len() != proc_info.args_decl.len() {
            return Err(TranslationError::InvalidNumberOfArguments(location, name.clone()));
        }

        // check if the type of each argument matches
        // the type of the destination procedure parameter

        for (arg_no, (arg_name, arg_decl)) in zip(args, &proc_info.args_decl).enumerate() {
            if let Some(arg_entry) = symbol_table.get_mut(arg_name) {

                // check type equality

                if matches!(arg_decl, ArgumentDeclaration::Var{..}) {
                    if let SymbolTableEntry::Var(arg) = arg_entry { // both are variables
            
                        // update the value in the symbol table

                        arg.value = ValueHeld::Dynamic;

                        // store the variable reference

                        let mut store_addr_code = translate_store_var_reference(arg.memloc, arg.is_ref, proc_info.mem_addr + 1 + arg_no as u64, symbol_table, register_states);
                        code.append(&mut store_addr_code);
                    } else {
                        return Err(TranslationError::VariableExpected(location, arg_name.clone()));
                    }
                } else if matches!(arg_decl, ArgumentDeclaration::Arr{..}) {
                    if let SymbolTableEntry::Arr(arg) = arg_entry { // both are arrays
            
                        // update the value in the symbol table

                        arg.value = ValueHeld::Dynamic;
                        
                        // store the variable reference

                        let mut store_addr_code = translate_store_var_reference(arg.memloc, arg.is_ref, proc_info.mem_addr + 1 + arg_no as u64, symbol_table, register_states);
                        code.append(&mut store_addr_code);
                    } else {
                        return Err(TranslationError::ArrayExpected(location, arg_name.clone()));
                    }
                } else {
                    panic!("Invalid argument declaration type in the symbol table");
                }
            } else {
                return Err(TranslationError::NoSuchVariable(location, arg_name.clone()));
            }
        }

        // store all variables currently kept in registers

        let variable_registers = [Register::C, Register::D, Register::E, Register::F, Register::G, Register::H];
        for register in variable_registers {
            if let RegisterState::Variable(id) = register_states.registers.get(&register).unwrap() {
                let mut var_store_code = store_variable_code(&id.clone(), &register, symbol_table, register_states);
                code.append(&mut var_store_code);

                register_states.registers.entry(register).and_modify(|e| *e = RegisterState::Noise);
            }
        }

        // load the return's storage address...

        let mut store_addr = translate_load_const(proc_info.mem_addr, &Register::B, register_states, symbol_table);
        code.append(&mut store_addr);

        // ...and store the return address there

        let mut return_addr_offset = translate_load_const(4, &Register::C, register_states, symbol_table);
        code.append(&mut return_addr_offset);

        add_command(&mut code, "STRK a");
        add_command(&mut code, "ADD c");
        add_command(&mut code, "STORE b");

        // jump to the address that begins the procedure

        add_command_string(&mut code, "JUMP ".to_owned() + &(proc_info.code_line_number).to_string());

    } else {
        return Err(TranslationError::NoSuchProcedure(location, name.clone()));
    }
    return Ok(code);
}

fn translate_equal(lhs: &Value, rhs: &Value, symbol_table: &mut SymbolTable, register_states: &mut RegisterStates, location: Location) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();
    
    // load the rhs value

    // check if a register is already holding the value

    let rhs_register;

    if let Value::Id(id) = rhs {
        if let Some(register) = register_states.scan(id) {

            // if the variable is already loaded, do nothing,

            rhs_register = register;

        } else {

            // if any of the conditions fail, load its value into the next register

            rhs_register = register_states.get_next();
            let mut rhs_code = translate_val(rhs, &rhs_register, symbol_table, register_states, location)?;
            code.append(&mut rhs_code);

        }
    } else {

        // if any of the conditions fail, load its value into the next register

        rhs_register = register_states.get_next();
        let mut rhs_code = translate_val(rhs, &rhs_register, symbol_table, register_states, location)?;
        code.append(&mut rhs_code);

    }

    // load the lhs value

    // check if a register is already holding the value

    let lhs_register;

    if let Value::Id(id) = lhs {
        if let Some(register) = register_states.scan(id) {

            // if the variable is already loaded, do nothing

            lhs_register = register;

        } else {
            
            // if any of the conditions fail, load its value into the next register
            
            lhs_register = register_states.get_next();
            let mut lhs_code = translate_val(lhs, &lhs_register, symbol_table, register_states, location)?;
            code.append(&mut lhs_code);

        }
    } else {
        
        // if any of the conditions fail, load its value into the next register
        
        lhs_register = register_states.get_next();
        let mut lhs_code = translate_val(lhs, &lhs_register, symbol_table, register_states, location)?;
        code.append(&mut lhs_code);

    }

    let comment = "condition".to_owned() + &format!("{:?}", lhs) + " = " + &format!("{:?}", rhs) + "";
    add_comment(&mut code, &comment);

    // then check for equality of the two values
    
    let mut comparison_code = Vec::new();
    
    
    add_command_string(&mut comparison_code, "GET ".to_owned() + register_to_string(&lhs_register));
    add_command_string(&mut comparison_code, "SUB ".to_owned() + register_to_string(&rhs_register));
    add_command(&mut comparison_code, "JPOS "); // blank jump

    add_command_string(&mut comparison_code, "GET ".to_owned() + register_to_string(&rhs_register));
    add_command_string(&mut comparison_code, "SUB ".to_owned() + register_to_string(&lhs_register));
    add_command(&mut comparison_code, "JPOS "); // blank jump
    code.append(&mut comparison_code);
    
    register_states.registers.entry(Register::A).and_modify(|e| *e = RegisterState::Noise);

    return Ok(code);
}

fn translate_not_equal(lhs: &Value, rhs: &Value, symbol_table: &mut SymbolTable, register_states: &mut RegisterStates, location: Location) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();
    
    // load the rhs value

    // check if a register is already holding the value

    let rhs_register;

    if let Value::Id(id) = rhs {
        if let Some(register) = register_states.scan(id) {

            // if the variable is already loaded, do nothing,

            rhs_register = register;

        } else {

            // if any of the conditions fail, load its value into the next register

            rhs_register = register_states.get_next();
            let mut rhs_code = translate_val(rhs, &rhs_register, symbol_table, register_states, location)?;
            code.append(&mut rhs_code);

        }
    } else {

        // if any of the conditions fail, load its value into the next register

        rhs_register = register_states.get_next();
        let mut rhs_code = translate_val(rhs, &rhs_register, symbol_table, register_states, location)?;
        code.append(&mut rhs_code);

    }

    // load the lhs value

    // check if a register is already holding the value

    let lhs_register;

    if let Value::Id(id) = lhs {
        if let Some(register) = register_states.scan(id) {

            // if the variable is already loaded, do nothing

            lhs_register = register;

        } else {
            
            // if any of the conditions fail, load its value into the next register
            
            lhs_register = register_states.get_next();
            let mut lhs_code = translate_val(lhs, &lhs_register, symbol_table, register_states, location)?;
            code.append(&mut lhs_code);

        }
    } else {
        
        // if any of the conditions fail, load its value into the next register
        
        lhs_register = register_states.get_next();
        let mut lhs_code = translate_val(lhs, &lhs_register, symbol_table, register_states, location)?;
        code.append(&mut lhs_code);

    }

    let comment = "condition".to_owned() + &format!("{:?}", lhs) + " != " + &format!("{:?}", rhs) + "";
    add_comment(&mut code, &comment);

    // then check for difference of the two values

    let mut comparison_code = Vec::new();

    add_command_string(&mut comparison_code, "GET ".to_owned() + register_to_string(&lhs_register));
    add_command_string(&mut comparison_code, "SUB ".to_owned() + register_to_string(&rhs_register));
    add_command(&mut comparison_code, "PUT b");

    add_command_string(&mut comparison_code, "GET ".to_owned() + register_to_string(&rhs_register));
    add_command_string(&mut comparison_code, "SUB ".to_owned() + register_to_string(&lhs_register));
    add_command(&mut comparison_code, "ADD b");
    add_command(&mut comparison_code, "JZERO "); // blank jump
    code.append(&mut comparison_code);

    register_states.registers.entry(Register::A).and_modify(|e| *e = RegisterState::Noise);
    register_states.registers.entry(Register::B).and_modify(|e| *e = RegisterState::Noise);
    
    return Ok(code);
}

fn translate_greater(lhs: &Value, rhs: &Value, symbol_table: &mut SymbolTable, register_states: &mut RegisterStates, location: Location) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    // load the rhs value

    // check if a register is already holding the value

    let rhs_register;

    if let Value::Id(id) = rhs {
        if let Some(register) = register_states.scan(id) {

            // if the variable is already loaded, do nothing,

            rhs_register = register;

        } else {

            // if any of the conditions fail, load its value into the next register

            rhs_register = register_states.get_next();
            let mut rhs_code = translate_val(rhs, &rhs_register, symbol_table, register_states, location)?;
            code.append(&mut rhs_code);

        }
    } else {

        // if any of the conditions fail, load its value into the next register

        rhs_register = register_states.get_next();
        let mut rhs_code = translate_val(rhs, &rhs_register, symbol_table, register_states, location)?;
        code.append(&mut rhs_code);

    }

    // load the lhs value

    // check if a register is already holding the value

    let lhs_register;

    if let Value::Id(id) = lhs {
        if let Some(register) = register_states.scan(id) {

            // if the variable is already loaded, do nothing

            lhs_register = register;

        } else {
            
            // if any of the conditions fail, load its value into the next register
            
            lhs_register = register_states.get_next();
            let mut lhs_code = translate_val(lhs, &lhs_register, symbol_table, register_states, location)?;
            code.append(&mut lhs_code);

        }
    } else {
        
        // if any of the conditions fail, load its value into the next register
        
        lhs_register = register_states.get_next();
        let mut lhs_code = translate_val(lhs, &lhs_register, symbol_table, register_states, location)?;
        code.append(&mut lhs_code);

    }

    let comment = "condition".to_owned() + &format!("{:?}", lhs) + " > " + &format!("{:?}", rhs) + "";
    add_comment(&mut code, &comment);

    // then check if lhs > rhs
    
    let mut comparison_code = Vec::new();

    add_command_string(&mut comparison_code, "GET ".to_owned() + register_to_string(&lhs_register));
    add_command_string(&mut comparison_code, "SUB ".to_owned() + register_to_string(&rhs_register));
    add_command(&mut comparison_code, "JZERO "); // blank jump
    code.append(&mut comparison_code);
    
    register_states.registers.entry(Register::A).and_modify(|e| *e = RegisterState::Noise);
    
    return Ok(code);
}

fn translate_lesser(lhs: &Value, rhs: &Value, symbol_table: &mut SymbolTable, register_states: &mut RegisterStates, location: Location) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();
    
    // load the rhs value

    // check if a register is already holding the value

    let rhs_register;

    if let Value::Id(id) = rhs {
        if let Some(register) = register_states.scan(id) {

            // if the variable is already loaded, do nothing,

            rhs_register = register;

        } else {

            // if any of the conditions fail, load its value into the next register

            rhs_register = register_states.get_next();
            let mut rhs_code = translate_val(rhs, &rhs_register, symbol_table, register_states, location)?;
            code.append(&mut rhs_code);

        }
    } else {

        // if any of the conditions fail, load its value into the next register

        rhs_register = register_states.get_next();
        let mut rhs_code = translate_val(rhs, &rhs_register, symbol_table, register_states, location)?;
        code.append(&mut rhs_code);

    }

    // load the lhs value

    // check if a register is already holding the value

    let lhs_register;

    if let Value::Id(id) = lhs {
        if let Some(register) = register_states.scan(id) {

            // if the variable is already loaded, do nothing

            lhs_register = register;

        } else {
            
            // if any of the conditions fail, load its value into the next register
            
            lhs_register = register_states.get_next();
            let mut lhs_code = translate_val(lhs, &lhs_register, symbol_table, register_states, location)?;
            code.append(&mut lhs_code);

        }
    } else {
        
        // if any of the conditions fail, load its value into the next register
        
        lhs_register = register_states.get_next();
        let mut lhs_code = translate_val(lhs, &lhs_register, symbol_table, register_states, location)?;
        code.append(&mut lhs_code);

    }

    let comment = "condition".to_owned() + &format!("{:?}", lhs) + " < " + &format!("{:?}", rhs) + "";
    add_comment(&mut code, &comment);

    // then check if lhs < rhs

    let mut comparison_code = Vec::new();

    add_command_string(&mut comparison_code, "GET ".to_owned() + register_to_string(&rhs_register));
    add_command_string(&mut comparison_code, "SUB ".to_owned() + register_to_string(&lhs_register));
    add_command(&mut comparison_code, "JZERO "); // blank jump
    code.append(&mut comparison_code);
    
    register_states.registers.entry(Register::A).and_modify(|e| *e = RegisterState::Noise);
    
    return Ok(code);
}

fn translate_greater_or_equal(lhs: &Value, rhs: &Value, symbol_table: &mut SymbolTable, register_states: &mut RegisterStates, location: Location) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();
    
    // load the rhs value

    // check if a register is already holding the value

    let rhs_register;

    if let Value::Id(id) = rhs {
        if let Some(register) = register_states.scan(id) {

            // if the variable is already loaded, do nothing,

            rhs_register = register;

        } else {

            // if any of the conditions fail, load its value into the next register

            rhs_register = register_states.get_next();
            let mut rhs_code = translate_val(rhs, &rhs_register, symbol_table, register_states, location)?;
            code.append(&mut rhs_code);

        }
    } else {

        // if any of the conditions fail, load its value into the next register

        rhs_register = register_states.get_next();
        let mut rhs_code = translate_val(rhs, &rhs_register, symbol_table, register_states, location)?;
        code.append(&mut rhs_code);

    }

    // load the lhs value

    // check if a register is already holding the value

    let lhs_register;

    if let Value::Id(id) = lhs {
        if let Some(register) = register_states.scan(id) {

            // if the variable is already loaded, do nothing

            lhs_register = register;

        } else {
            
            // if any of the conditions fail, load its value into the next register
            
            lhs_register = register_states.get_next();
            let mut lhs_code = translate_val(lhs, &lhs_register, symbol_table, register_states, location)?;
            code.append(&mut lhs_code);

        }
    } else {
        
        // if any of the conditions fail, load its value into the next register
        
        lhs_register = register_states.get_next();
        let mut lhs_code = translate_val(lhs, &lhs_register, symbol_table, register_states, location)?;
        code.append(&mut lhs_code);

    }

    let comment = "condition".to_owned() + &format!("{:?}", lhs) + " >= " + &format!("{:?}", rhs) + "";
    add_comment(&mut code, &comment);

    // then check if lhs >= rhs

    let mut comparison_code = Vec::new();

    add_command_string(&mut comparison_code, "GET ".to_owned() + register_to_string(&rhs_register));
    add_command_string(&mut comparison_code, "SUB ".to_owned() + register_to_string(&lhs_register));
    add_command(&mut comparison_code, "JPOS "); // blank jump
    code.append(&mut comparison_code);
    
    register_states.registers.entry(Register::A).and_modify(|e| *e = RegisterState::Noise);
    
    return Ok(code);
}

fn translate_lesser_or_equal(lhs: &Value, rhs: &Value, symbol_table: &mut SymbolTable, register_states: &mut RegisterStates, location: Location) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();
    
    // load the rhs value

    // check if a register is already holding the value

    let rhs_register;

    if let Value::Id(id) = rhs {
        if let Some(register) = register_states.scan(id) {

            // if the variable is already loaded, do nothing,

            rhs_register = register;

        } else {

            // if any of the conditions fail, load its value into the next register

            rhs_register = register_states.get_next();
            let mut rhs_code = translate_val(rhs, &rhs_register, symbol_table, register_states, location)?;
            code.append(&mut rhs_code);

        }
    } else {

        // if any of the conditions fail, load its value into the next register

        rhs_register = register_states.get_next();
        let mut rhs_code = translate_val(rhs, &rhs_register, symbol_table, register_states, location)?;
        code.append(&mut rhs_code);

    }

    // load the lhs value

    // check if a register is already holding the value

    let lhs_register;

    if let Value::Id(id) = lhs {
        if let Some(register) = register_states.scan(id) {

            // if the variable is already loaded, do nothing

            lhs_register = register;

        } else {
            
            // if any of the conditions fail, load its value into the next register
            
            lhs_register = register_states.get_next();
            let mut lhs_code = translate_val(lhs, &lhs_register, symbol_table, register_states, location)?;
            code.append(&mut lhs_code);

        }
    } else {
        
        // if any of the conditions fail, load its value into the next register
        
        lhs_register = register_states.get_next();
        let mut lhs_code = translate_val(lhs, &lhs_register, symbol_table, register_states, location)?;
        code.append(&mut lhs_code);

    }

    let comment = "condition".to_owned() + &format!("{:?}", lhs) + " <= " + &format!("{:?}", rhs) + "";
    add_comment(&mut code, &comment);

    // then check if lhs <= rhs
    
    let mut comparison_code = Vec::new();

    add_command_string(&mut comparison_code, "GET ".to_owned() + register_to_string(&lhs_register));
    add_command_string(&mut comparison_code, "SUB ".to_owned() + register_to_string(&rhs_register));
    add_command(&mut comparison_code, "JPOS "); // blank jump
    code.append(&mut comparison_code);
    
    register_states.registers.entry(Register::A).and_modify(|e| *e = RegisterState::Noise);
    
    return Ok(code);
}

fn translate_condition(condition: &Condition, symbol_table: &mut SymbolTable, register_states: &mut RegisterStates, location: Location) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    match condition {

        // translate the corresponding condition

        Condition::Equal(lhs, rhs) => {
            let mut condition_code = translate_equal(lhs, rhs, symbol_table, register_states, location)?;
            code.append(&mut condition_code);
        },
        Condition::NotEqual(lhs, rhs) => {
            let mut condition_code = translate_not_equal(lhs, rhs, symbol_table, register_states, location)?;
            code.append(&mut condition_code);
        },
        Condition::Greater(lhs, rhs) => {
            let mut condition_code = translate_greater(lhs, rhs, symbol_table, register_states, location)?;
            code.append(&mut condition_code);
        },
        Condition::Lesser(lhs, rhs) => {
            let mut condition_code = translate_lesser(lhs, rhs, symbol_table, register_states, location)?;
            code.append(&mut condition_code);
        },
        Condition::GreaterOrEqual(lhs, rhs) => {
            let mut condition_code = translate_greater_or_equal(lhs, rhs, symbol_table, register_states, location)?;
            code.append(&mut condition_code);
        },
        Condition::LesserOrEqual(lhs, rhs) => {
            let mut condition_code = translate_lesser_or_equal(lhs, rhs, symbol_table, register_states, location)?;
            code.append(&mut condition_code);
        },
    }

    return Ok(code);
}

fn translate_if(condition: &Condition, commands: &Commands, symbol_table: &mut SymbolTable, function_table: &FunctionTable, curr_line: usize, curr_proc: Option<&Pidentifier>, register_states: &mut RegisterStates, location: Location) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    println!("{:?}", register_states);

    // translate the condition code

    let mut condition_code = translate_condition(condition, symbol_table, register_states, location)?;

    // translate the if commands

    let mut commands_code = translate_commands(commands, symbol_table, function_table, curr_line + condition_code.len(), curr_proc, register_states)?;

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

    // RESET REGISTER STATES

    let mut reset_code = reset_register_memory(symbol_table, register_states);
    code.append(&mut reset_code);

    return Ok(code);
}

fn translate_if_else(condition: &Condition, if_commands: &Commands, else_commands: &Commands, symbol_table: &mut SymbolTable, function_table: &FunctionTable, curr_line: usize, curr_proc: Option<&Pidentifier>, register_states: &mut RegisterStates, location: Location) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    // translate the condition code

    let mut condition_code = translate_condition(condition, symbol_table, register_states, location)?;

    // translate the if commands

    let mut if_commands_code = translate_commands(if_commands, symbol_table, function_table, curr_line + condition_code.len(), curr_proc, &mut register_states.clone())?;

    // translate the else commands

    let mut else_commands_code = translate_commands(else_commands, symbol_table, function_table, curr_line + condition_code.len() + if_commands_code.len() + 1, curr_proc, register_states)?;

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

    // RESET REGISTER STATES

    let mut reset_code = reset_register_memory(symbol_table, register_states);
    code.append(&mut reset_code);

    return Ok(code);
}

fn translate_while(condition: &Condition, commands: &Commands, symbol_table: &mut SymbolTable, function_table: &FunctionTable, mut curr_line: usize, curr_proc: Option<&Pidentifier>, register_states: &mut RegisterStates, location: Location) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    // RESET REGISTER STATES

    let mut begin_reset_code = reset_register_memory(symbol_table, register_states);

    code.append(&mut begin_reset_code);

    curr_line += code.len();

    // translate the condition code

    let mut condition_code = translate_condition(condition, symbol_table, register_states, location)?;

    // translate the loop commands

    let mut commands_code = translate_commands(commands, symbol_table, function_table, curr_line + condition_code.len(), curr_proc, register_states)?;
 
    // reset registers at the end of the loop

    let mut end_reset_code = reset_register_memory(symbol_table, register_states);
    commands_code.append(&mut end_reset_code);

    // jump to the beginning of the loop at the end of commands block

    add_command_string(&mut commands_code, "JUMP ".to_owned() + &(curr_line.to_string()));

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

fn translate_repeat(commands: &Commands, condition: &Condition, symbol_table: &mut SymbolTable, function_table: &FunctionTable, mut curr_line: usize, curr_proc: Option<&Pidentifier>, register_states: &mut RegisterStates, location: Location) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();
    
    // RESET REGISTER STATES

    let mut begin_reset_code = reset_register_memory(symbol_table, register_states);

    code.append(&mut begin_reset_code);

    curr_line += code.len();

    // translate the loop commands

    let mut commands_code = translate_commands(commands, symbol_table, function_table, curr_line, curr_proc, register_states)?;
    
    // reset registers at the end of the loop

    let mut end_reset_code = reset_register_memory(symbol_table, register_states);
    commands_code.append(&mut end_reset_code);

    // translate the condition code

    let mut condition_code = translate_condition(condition, symbol_table, register_states, location)?;

    // fill the blank jumps in condition code

    if matches!(condition, Condition::Equal{..}) {

        // equality condition requires filling an extra jump

        let blank_jump_idx = condition_code.len() - 4;
        condition_code[blank_jump_idx] += &((curr_line).to_string());
    }

    let blank_jump_idx = condition_code.len() - 1;
    condition_code[blank_jump_idx] += &((curr_line).to_string());

    // join the partial codes

    code.append(&mut commands_code);
    code.append(&mut condition_code);

    return Ok(code);
}

// read user-inputted value and store it at the address of the Identifier
fn translate_read(id: &Identifier, symbol_table: &mut SymbolTable, register_states: &mut RegisterStates, location: Location) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    // read an input value into register A

    add_command(&mut code, "READ");

    // move the value into the next register

    let register = register_states.get_next();
    register_states.registers.entry(register.clone()).and_modify(|e| *e = RegisterState::Variable(id.clone()));
    add_command_string(&mut code, "PUT ".to_owned() + register_to_string(&register));

    return Ok(code);
}

// write the specified Value on the output
fn translate_write(value: &Value, symbol_table: &mut SymbolTable, register_states: &mut RegisterStates, location: Location) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();
    
    if let Value::Id(id) = value {
        if let Some(register) = register_states.scan(id) {
            
            // if the value already is in a register, copy the value from there

            add_command_string(&mut code, "GET ".to_owned() + register_to_string(&register));
        } else {

            // if any of the conditions fail, load its value into register A

            let mut val_code = translate_val(value, &Register::A, symbol_table, register_states, location)?;
            code.append(&mut val_code);

        }
    } else {

        // if any of the conditions fail, load its value into register A

        let mut val_code = translate_val(value, &Register::A, symbol_table, register_states, location)?;
        code.append(&mut val_code);

    }

    // write the value on the output

    add_command(&mut code, "WRITE");

    return Ok(code);
}

// generate appropriate virtual machine code for the specified commands
fn translate_commands(commands: &Commands, symbol_table: &mut SymbolTable, function_table: &FunctionTable, curr_line: usize, curr_proc: Option<&Pidentifier>, register_states: &mut RegisterStates) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    for command in commands {

        // translate the command

        match command {
            Command::Assignment(id, expr, location) => {
                let mut command_code = translate_assignment(id, expr, symbol_table, curr_line + code.len(), register_states, *location)?;
                let comment = "--- ".to_owned() + &format!("{:?}", command) + " ---";
                add_comment(&mut command_code, &comment);
                code.append(&mut command_code);
            },
            Command::If(condition, commands, location) => {
                let mut command_code = translate_if(condition, commands, symbol_table, function_table, curr_line + code.len(), curr_proc, register_states, *location)?;
                let comment = "--- ".to_owned() + &format!("{:?}", command) + " ---";
                add_comment(&mut command_code, &comment);
                code.append(&mut command_code);
            }
            Command::IfElse(condition, if_commands, else_commands, location) => {
                let mut command_code = translate_if_else(condition, if_commands, else_commands, symbol_table, function_table, curr_line + code.len(), curr_proc, register_states, *location)?;
                let comment = "--- ".to_owned() + &format!("{:?}", command) + " ---";
                add_comment(&mut command_code, &comment);
                code.append(&mut command_code);
            }
            Command::While(condition, commands, location) => {
                let mut command_code = translate_while(condition, commands, symbol_table, function_table, curr_line + code.len(), curr_proc, register_states, *location)?;
                let comment = "--- ".to_owned() + &format!("{:?}", command) + " ---";
                add_comment(&mut command_code, &comment);
                code.append(&mut command_code);
            }
            Command::Repeat(commands, condition, location) => {
                let mut command_code = translate_repeat(commands, condition, symbol_table, function_table, curr_line + code.len(), curr_proc, register_states, *location)?;
                let comment = "--- ".to_owned() + &format!("{:?}", command) + " ---";
                add_comment(&mut command_code, &comment);
                code.append(&mut command_code);
            }
            Command::ProcedureCall(proc_call, location) => {
                let mut command_code = translate_proc_call(&proc_call.name, &proc_call.args, symbol_table, function_table, curr_proc, register_states, *location)?;
                let comment = "--- ".to_owned() + &format!("{:?}", command) + " ---";
                add_comment(&mut command_code, &comment);
                code.append(&mut command_code);
            }
            Command::Read(id, location) => {
                let mut command_code = translate_read(id, symbol_table, register_states, *location)?;
                let comment = "--- ".to_owned() + &format!("{:?}", command) + " ---";
                add_comment(&mut command_code, &comment);
                code.append(&mut command_code);
            }
            Command::Write(value, location) => {
                let mut command_code = translate_write(value, symbol_table, register_states, *location)?;
                let comment = "--- ".to_owned() + &format!("{:?}", command) + " ---";
                add_comment(&mut command_code, &comment);
                code.append(&mut command_code);
            }
        }
    }
    return Ok(code);
}

fn translate_procedure(procedure: &Procedure, function_table: &mut FunctionTable, mut curr_mem_byte: u64, curr_line: usize, register_states: &mut RegisterStates) -> Result<(Vec<String>, u64), TranslationError> {
    let mut code = Vec::new();

    // add the procedure to the function table

    malloc_proc(&procedure.proc_head, function_table, curr_line, curr_mem_byte, procedure.location)?;

    // create the procedure's symbol table

    let mut symbol_table = SymbolTable::new();

    // insert the return address object into the symbol table

    symbol_table.insert(".return".to_owned(), SymbolTableEntry::Ret(ReturnLocation::new(curr_mem_byte)));
    curr_mem_byte += 1;

    // allocate memory for the argument references and procedure declarations

    let curr_mem_byte = malloc_args(curr_mem_byte, &procedure.proc_head.args_decl, &mut symbol_table, procedure.location)?;
    let next_mem_byte = malloc(curr_mem_byte, &procedure.declarations, &mut symbol_table, procedure.location)?;
    //println!("{} Symbol table: {:?}", &procedure.proc_head.name, symbol_table);

    // translate the procedure commands

    let mut proc_code = translate_commands(&procedure.commands, &mut symbol_table, &function_table, curr_line, Some(&procedure.proc_head.name), register_states)?;
    code.append(&mut proc_code);

    // attach return code

    let mut ret_code = translate_return(&mut symbol_table, register_states);
    code.append(&mut ret_code);


    return Ok((code, next_mem_byte));
}

fn translate_main(main: &Main, function_table: &FunctionTable, curr_mem_byte: u64, curr_line: usize, register_states: &mut RegisterStates) -> Result<Vec<String>, TranslationError> {
    let mut code = Vec::new();

    // create main's symbol table

    let mut symbol_table = SymbolTable::new();

    // allocate memory for the declarations
    
    let _next_mem_byte = malloc(curr_mem_byte, &main.declarations, &mut symbol_table, main.location)?;
    //println!("Main Symbol table: {:?}", symbol_table);

    // translate the Main commands

    let mut main_code = translate_commands(&main.commands, &mut symbol_table, function_table, curr_line, None, register_states)?;
    code.append(&mut main_code);

    return Ok(code);

}

// TODO: check variable initialisation
pub fn translate(ast: ProgramAll) -> Result<Vec<String>, TranslationError> {
    let mut register_states = RegisterStates::new();

    let mut code = Vec::new();

    // add blank JUMP (jump to main) if there are any procedures
    
    if !ast.procedures.is_empty() {
        add_command(&mut code, "JUMP <MAIN ADDRESS>");
    }

    // translate the code

    let mut function_table = FunctionTable::new();
    let mut curr_mem_byte = 0;

    // translate the procedures into code

    for procedure in &ast.procedures {

        // translate the the procedure

        let (mut proc_code, next_mem_byte) = translate_procedure(&procedure, &mut function_table, curr_mem_byte, code.len(), &mut register_states)?;
        add_comment(&mut proc_code, &procedure.proc_head.name);
        code.append(&mut proc_code);

        // update the location of the next free memory byte

        curr_mem_byte = next_mem_byte;
    }

    // fill the jump to main if it has been added

    if !ast.procedures.is_empty() {
        let main_jump_code = "JUMP ".to_owned() + &code.len().to_string();
        code[0] = main_jump_code;
    }

    // translate main into code

    let mut main_code = translate_main(&ast.main, &function_table, curr_mem_byte, code.len(), &mut register_states)?;
    add_comment(&mut main_code, ">>> Main <<<");
    code.append(&mut main_code);

    // some simple verifications of the code

    for i in 0..code.len() {
        if code[i].starts_with("#") {
            panic!("Comment at the beginning of line");
        }
        if code[i].ends_with("\n") {
            panic!("No newline symbol at the end of line");
        }
    }

    // finish the program

    add_command(&mut code, "HALT");

    return Ok(code);
}
