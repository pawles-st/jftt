use crate::ast::*;
use super::translation_structures::*;
use std::collections::HashMap;

type ProcCallCount = HashMap<String, usize>;

fn check_calls(commands: &Commands, function_calls: &mut ProcCallCount) -> Result<(), TranslationError> {
    for command in commands {
        match command {
            Command::ProcedureCall(proc_call, location) => {
                match function_calls.get_mut(&proc_call.name) {
                    Some(no_proc_calls) => *no_proc_calls += 1,
                    None => return Err(TranslationError::NoSuchProcedure(*location, proc_call.name.clone())),
                }
            },
            Command::IfElse(_, if_commands, else_commands, _) => {
                check_calls(if_commands, function_calls)?;
                check_calls(else_commands, function_calls)?;
            },
            Command::If(_, commands, _) => {
                check_calls(commands, function_calls)?;
            },
            Command::While(_, commands, _) => {
                check_calls(commands, function_calls)?;
            },
            Command::Repeat(commands, _, _) => {
                check_calls(commands, function_calls)?;
            },
            _ => {},
        }
    }

    Ok(())
}

pub fn transform(ast: &mut ProgramAll) -> Result<(), TranslationError> {
    
    // perform function inlining

    let mut function_calls = ProcCallCount::new();

    // scan procedures for calls

    for procedure in &ast.procedures {

        // add the procedure to the list

        function_calls.insert(procedure.proc_head.name.clone(), 0);

        check_calls(&procedure.commands, &mut function_calls)?;
    }

    // scan main for calls
    
    check_calls(&ast.main.commands, &mut function_calls)?;

    // remove procedures that are never called

    ast.procedures.retain(|procedure| *function_calls.get(&procedure.proc_head.name).unwrap() > 0);

    // expand procedures which are called only once

    

    Ok(())
}
