use crate::ast::*;
use super::translation_structures::*;
use std::collections::HashMap;
use std::iter::zip;

type FunctionCallTable = HashMap<String, usize>;

// search the list of commands for a call to a procedure and count these
fn check_calls(commands: &Commands, function_calls: &mut FunctionCallTable) -> Result<(), TranslationError> {
    for command in commands {
        match command {
            Command::ProcedureCall(proc_call, location) => {

                // check if the call is assigned to an allowed procedure

                match function_calls.get_mut(&proc_call.name) {
                    Some(no_proc_calls) => *no_proc_calls += 1, // increase the count of calls for the appropriate procedure
                    None => return Err(TranslationError::NoSuchProcedure(*location, proc_call.name.clone())),
                }
            },
            Command::IfElse(_, if_commands, else_commands, _) => {

                // recursively check the commands inside the if block
                
                check_calls(if_commands, function_calls)?;
                
                // recursively check the commands inside the else block
                
                check_calls(else_commands, function_calls)?;
            },
            Command::If(_, commands, _) => {
                
                // recursively check the commands inside the if block
                
                check_calls(commands, function_calls)?;
            },
            Command::While(_, commands, _) => {
                
                // recursively check the commands inside the while block
                
                check_calls(commands, function_calls)?;
            },
            Command::Repeat(commands, _, _) => {
                
                // recursively check the commands inside the repeat block
                
                check_calls(commands, function_calls)?;
            },
            _ => {}, // ignore any other commands
        }
    }

    Ok(())
}

// count the number of times each procedure is called in the source code
fn count_calls(ast: &ProgramAll) -> Result<FunctionCallTable, TranslationError> {

    let mut function_calls = FunctionCallTable::new();

    // scan all procedures for calls

    for procedure in &ast.procedures {

        // add the procedure to the function call table

        function_calls.insert(procedure.proc_head.name.clone(), 0);

        // search its commands list for procedure calls

        check_calls(&procedure.commands, &mut function_calls)?;
    }

    // scan main for calls
    
    check_calls(&ast.main.commands, &mut function_calls)?;

    return Ok(function_calls);
}

fn total_commands_count(commands: &Commands) -> usize {
    let mut commands_remaining = commands.clone();
    let mut total_commands = 0;

    while !commands_remaining.is_empty() {
        total_commands += 1;
        let mut curr_command = commands_remaining.pop().unwrap();
        match curr_command {
            Command::IfElse(_, ref mut if_commands, ref mut else_commands, _) => {
                commands_remaining.append(if_commands);
                commands_remaining.append(else_commands);
            },
            Command::If(_, ref mut if_commands, _) => {
                commands_remaining.append(if_commands);
            },
            Command::While(_, ref mut while_commands, _) => {
                commands_remaining.append(while_commands);
            },
            Command::Repeat(ref mut repeat_commands, _, _) => {
                commands_remaining.append(repeat_commands);
            },
            _ => {},
        }
    }

    return total_commands;
}

fn replace_id(id: &mut Identifier, from: &Pidentifier, to: &Pidentifier) {
    match id {
        Identifier::Pid(pid) => {
            if pid == from {
                *id = Identifier::Pid(to.clone());
            }
        },
        Identifier::ArrNum(pid, num) => {
            if pid == from {
                *id = Identifier::ArrNum(to.clone(), *num);
            }
        },
        Identifier::ArrPid(arrpid, numpid) => {
            let num_copy = numpid.clone();
            let mut arr_copy = arrpid.clone();
            if arr_copy == *from {
                *id = Identifier::ArrPid(to.clone(), num_copy.clone());
                arr_copy = to.clone();
            }
            if num_copy == *from {
                *id = Identifier::ArrPid(arr_copy.clone(), to.clone());
            }
        }
    }
}

fn replace_value(value: &mut Value, from: &Pidentifier, to: &Pidentifier) {
    if let Value::Id(id) = value {
        replace_id(id, from, to);
    }
}

fn replace_expr(expr: &mut Expression, from: &Pidentifier, to: &Pidentifier) {
    match expr {
        Expression::Val(value) => {
            replace_value(value, from, to);
        },
        Expression::Add(lhs, rhs) => {
            replace_value(lhs, from, to);
            replace_value(rhs, from, to);
        }
        Expression::Sub(lhs, rhs) => {
            replace_value(lhs, from, to);
            replace_value(rhs, from, to);
        }
        Expression::Mul(lhs, rhs) => {
            replace_value(lhs, from, to);
            replace_value(rhs, from, to);
        }
        Expression::Div(lhs, rhs) => {
            replace_value(lhs, from, to);
            replace_value(rhs, from, to);
        }
        Expression::Mod(lhs, rhs) => {
            replace_value(lhs, from, to);
            replace_value(rhs, from, to);
        }
    }
}

fn replace_condition(condition: &mut Condition, from: &Pidentifier, to: &Pidentifier) {
    match condition {
        Condition::Equal(ref mut lhs, ref mut rhs) => {
            replace_value(lhs, from, to);
            replace_value(rhs, from, to);
        },
        Condition::NotEqual(ref mut lhs, ref mut rhs) => {
            replace_value(lhs, from, to);
            replace_value(rhs, from, to);
        },
        Condition::Greater(ref mut lhs, ref mut rhs) => {
            replace_value(lhs, from, to);
            replace_value(rhs, from, to);
        },
        Condition::Lesser(ref mut lhs, ref mut rhs) => {
            replace_value(lhs, from, to);
            replace_value(rhs, from, to);
        },
        Condition::GreaterOrEqual(ref mut lhs, ref mut rhs) => {
            replace_value(lhs, from, to);
            replace_value(rhs, from, to);
        },
        Condition::LesserOrEqual(ref mut lhs, ref mut rhs) => {
            replace_value(lhs, from, to);
            replace_value(rhs, from, to);
        },
    }
}

fn replace_proc_call(proc_call: &mut ProcCall, from: &Pidentifier, to: &Pidentifier) {
    for arg_name in proc_call.args.iter_mut() {
        if arg_name == from {
            *arg_name = to.clone();
        }
    }
}

// replace all procedure parameters with the arguments provided in the call 
fn replace_parameters(dest_procedure: &mut Procedure, curr_proc_call_args: &Arguments) {
    for (dest_args_decl, curr_arg) in zip(&mut dest_procedure.proc_head.args_decl, curr_proc_call_args) {
        match dest_args_decl {
            ArgumentDeclaration::Var(dest_arg) => {
                replace(&mut dest_procedure.commands, dest_arg, &curr_arg);
                *dest_args_decl = ArgumentDeclaration::Var(curr_arg.clone());
            },
            ArgumentDeclaration::Arr(dest_arg) => {
                replace(&mut dest_procedure.commands, dest_arg, &curr_arg);
                *dest_args_decl = ArgumentDeclaration::Arr(curr_arg.clone());
            },
        }
    }
}

// rename all procedure declarations if they conflict with the caller's
fn replace_declarations(dest_procedure: &mut Procedure, curr_proc_args_decls: Option<&ArgumentDeclarations>, curr_proc_decls: &mut Declarations) {
    for dest_arg in dest_procedure.declarations.iter_mut() {

        // extract the current declaration variable name

        let original_dest_pid = match dest_arg {
            Declaration::Var(pid) => {
                pid
            },
            Declaration::Arr(pid, _) => {
                pid
            },
        };
        let mut new_dest_pid = original_dest_pid.clone();

        // check caller's argument declarations, if any

        if let Some(args_decls) = curr_proc_args_decls {
            loop {
                if args_decls.iter().find(|decl| {
                    match decl {
                        ArgumentDeclaration::Var(arg_pid) => {
                            new_dest_pid == *arg_pid
                        },
                        ArgumentDeclaration::Arr(arg_pid) => {
                            new_dest_pid == *arg_pid
                        },
                    }
                }).is_some() {
                    new_dest_pid.insert_str(0, "_");
                } else {
                    break;
                }
            }
        }

        // check the caller's declarations

        loop {
            if curr_proc_decls.iter().find(|decl| {
                match decl {
                    Declaration::Var(arg_pid) => {
                        new_dest_pid == *arg_pid
                    },
                    Declaration::Arr(arg_pid, _) => {
                        new_dest_pid == *arg_pid
                    },
                }
            }).is_some() {
                new_dest_pid.insert_str(0, "_");
            } else {
                break;
            }
        }

        // replace the variable names in the procedure body and declarations if needed

        if *original_dest_pid != new_dest_pid {
            replace(&mut dest_procedure.commands, original_dest_pid, &new_dest_pid);
            match dest_arg {
                Declaration::Var(_) => {
                    *dest_arg = Declaration::Var(new_dest_pid);
                },
                Declaration::Arr(_, _) => {
                    *dest_arg = Declaration::Var(new_dest_pid);
                },
            };
        }
    }
}

// TODO: check repeated declarations
// replace all variable name usages with the other variable name
fn replace(commands: &mut Commands, from: &Pidentifier, to: &Pidentifier) {
    for command in commands.iter_mut() {
        match command {
            Command::Assignment(ref mut id, ref mut expr, _) => {
                replace_id(id, from, to);
                replace_expr(expr, from, to);
            },
            Command::IfElse(ref mut condition, ref mut if_commands, ref mut else_commands, _) => {
                replace_condition(condition, from, to);
                replace(if_commands, from, to);
                replace(else_commands, from, to);
            },
            Command::If(ref mut condition, ref mut if_commands, _) => {
                replace_condition(condition, from, to);
                replace(if_commands, from, to);
            },
            Command::While(ref mut condition, ref mut while_commands, _) => {
                replace_condition(condition, from, to);
                replace(while_commands, from, to);
            },
            Command::Repeat(ref mut repeat_commands, ref mut condition, _) => {
                replace(repeat_commands, from, to);
                replace_condition(condition, from, to);
            },
            Command::ProcedureCall(ref mut proc_call, _) => {
                replace_proc_call(proc_call, from, to);
            },
            Command::Read(ref mut id, _) => {
                replace_id(id, from, to);
            },
            Command::Write(ref mut value, _) => {
                replace_value(value, from, to);
            },
        }
    }
}

// expand all proc calls in the commands list which meet the required criteria
fn expand_procedures(procedures: &[Procedure], curr_proc_head: Option<&ProcHead>, curr_proc_declarations: &mut Declarations, commands: &mut Commands, function_calls: &FunctionCallTable) -> Result<(), TranslationError> {
    let mut proc_calls_replacements = Vec::new();

    // search the commands list for proc calls that meet the criteria

    for (command_idx, command) in commands.iter_mut().enumerate() {
        match command {
            Command::ProcedureCall(proc_call, location) => {

                // check for recurrence

                if let Some(proc_head) = curr_proc_head {
                    if proc_call.name == proc_head.name {
                        return Err(TranslationError::RecurrenceNotAllowed(*location, proc_call.name.clone()));
                    }
                }

                // find the procedure the call refers to...

                match procedures.iter().find(|&procedure| procedure.proc_head.name == proc_call.name) {
                    Some(procedure) => {

                        // ...and check if it meets the expansion criteria

                        let calls_count = *function_calls.get(&procedure.proc_head.name).unwrap();

                        if calls_count == 1 || total_commands_count(&procedure.commands) * calls_count < 20 {

                            // create a copy of the destination procedure and then modify its body

                            let mut dest_proc = procedure.clone();

                            // rename procedure declarations when needed to avoid conflicts

                            replace_declarations(&mut dest_proc, curr_proc_head.map_or(None, |head| Some(&head.args_decl)), curr_proc_declarations);

                            // replace all uses of argument parameters with the call variables

                            replace_parameters(&mut dest_proc, &proc_call.args.iter().map(|arg| arg.to_owned() + "'").collect());

                            // remove the chenge marks from the replaced names

                            replace_parameters(&mut dest_proc, &proc_call.args);

                            // store the procedure body for later expansion

                            proc_calls_replacements.push((command_idx, dest_proc.commands));

                            // copy the procedure declarations into the caller

                            curr_proc_declarations.append(&mut dest_proc.declarations);
                        }
                    },
                    None => return Err(TranslationError::NoSuchProcedure(*location, proc_call.name.clone())),
                }
            },
            Command::IfElse(_, ref mut if_commands, ref mut else_commands, _) => {
                
                // recursively check the commands inside the if block
                
                expand_procedures(procedures, curr_proc_head, curr_proc_declarations, if_commands, function_calls)?;
                
                // recursively check the commands inside the else block
                
                expand_procedures(procedures, curr_proc_head, curr_proc_declarations, else_commands, function_calls)?;
            },
            Command::If(_, ref mut commands, _) => {
                
                // recursively check the commands inside the if block
                
                expand_procedures(procedures, curr_proc_head, curr_proc_declarations, commands, function_calls)?;
            },
            Command::While(_, ref mut commands, _) => {
                
                // recursively check the commands inside the while block
                
                expand_procedures(procedures, curr_proc_head, curr_proc_declarations, commands, function_calls)?;
            },
            Command::Repeat(ref mut commands, _, _) => {
                
                // recursively check the commands inside the repeat block
                
                expand_procedures(procedures, curr_proc_head, curr_proc_declarations, commands, function_calls)?;
            },
            _ => {},
        }
    }

    // expand the encountered proc calls' bodies
    
    let mut offset = 0;
    for (command_idx, proc_commands) in proc_calls_replacements {
        let next_offset = proc_commands.len() - 1;
        commands.splice(command_idx+offset..=command_idx+offset, proc_commands);
        offset += next_offset;
    }
    //println!("PROCEDURE: {:?}", curr_proc_head.map_or("Main", |head| &head.name));
    //println!("curr_proc_decl: {:?}", curr_proc_declarations);
    //println!("curr_proc_commands: {:?}", commands);

    Ok(())
}

// expand all source code proc calls which meet the required criteria
fn expand_procedures_all(ast: &mut ProgramAll, function_calls: &FunctionCallTable) -> Result<(), TranslationError> {

    // expand calls inside each procedure

    for idx in 0..ast.procedures.len() {
        let (prev_procedures, remaining_procedures) = ast.procedures.split_at_mut(idx);
        let Procedure{proc_head: ref procedure_head, declarations: ref mut procedure_declarations, commands: ref mut procedure_commands, location: _} = &mut remaining_procedures[0];

        expand_procedures(&prev_procedures, Some(procedure_head), procedure_declarations, procedure_commands, &function_calls)?;
    }

    // expand calls inside main

    expand_procedures(&ast.procedures, None, &mut ast.main.declarations, &mut ast.main.commands, &function_calls)?;

    Ok(())
}

// transform the source code's AST for more effective compilation
pub fn transform(ast: &mut ProgramAll) -> Result<(), TranslationError> {

    // count the number of times each procedure is called

    let function_calls = count_calls(ast)?;

    // remove procedures that are never called

    ast.procedures.retain(|procedure| *function_calls.get(&procedure.proc_head.name).unwrap() > 0);

    // expand procedures which are called only once

    expand_procedures_all(ast, &function_calls)?;
    
    let function_calls = count_calls(ast)?;
    ast.procedures.retain(|procedure| *function_calls.get(&procedure.proc_head.name).unwrap() > 0);

    Ok(())
}
