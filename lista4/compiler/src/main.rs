use std::fs;
use std::io;
use std::io::Write;
use std::env;

use lalrpop_util::lalrpop_mod;
use grammar::ProgramAllParser;
use translation::translate;
use translation::translation_structures::TranslationError;

pub mod err;
pub mod ast;
pub mod translation;
lalrpop_mod!(pub grammar);

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: /path/to/programme <input-file> <output-file>");
        std::process::exit(1);
    }

    let program = fs::read_to_string(&args[1])?;
    match ProgramAllParser::new().parse(&program) {
        Ok(ast) => {
            //println!("Parsing succeeded!\nAST: {:?}", ast);
            match translate(ast) {
                Ok(code) => {
                    let all_code = code
                    .iter()
                    .fold(Vec::new(), |mut all_code, line| {
                        writeln!(&mut all_code, "{}", line).unwrap();
                        all_code
                    });
                    fs::write(&args[2], all_code)?;
                }
                Err(e) => match e {
                    TranslationError::NoSuchVariable(location, name) => {eprintln!("Error: No such variable: \"{}\" at bytes ({}, {})", name, location.0, location.1)},
                    TranslationError::NoSuchProcedure(location, name) => {eprintln!("Error: No such procedure: \"{}\" at bytes ({}, {})", name, location.0, location.1)},
                    TranslationError::RepeatedDeclaration(location, name) => {eprintln!("Error: Repeated declaration of \"{}\" at bytes ({}, {})", name, location.0, location.1)},
                    TranslationError::NotAnArray(location, name) => {eprintln!("Error: The variable \"{}\" at bytes ({}, {}) has not been declared as an array.\nHELP: remove the indexing {name}[...]", name, location.0, location.1)},
                    TranslationError::NoArrayIndex(location, name) => {eprintln!("Error: The variable: \"{}\" at bytes ({}, {}) has been declared as array, but no indexing was found.\nHELP: add indexing {name}[...]", name, location.0, location.1)},
                    TranslationError::ArrayExpected(location, name) => {eprintln!("Error: Expected an array variable, but got single variable \"{}\" at bytes ({}, {})", name, location.0, location.1)},
                    TranslationError::VariableExpected(location, name) => {eprintln!("Error: Expected a single variable, but got array variable \"{}\" at bytes ({}, {})", name, location.0, location.1)},
                    TranslationError::RecurrenceNotAllowed(location, name) => {eprintln!("Error: Recurrence in NOT allowed: invoking procedure \"{}\" inside itself at bytes ({}, {})", name, location.0, location.1)},
                    TranslationError::InvalidNumberOfArguments(location, name) => {eprintln!("Error: Invalid number of arguments found while trying to call \"{}\" at bytes ({}, {})", name, location.0, location.1)},
                }
            }
        },
        Err(e) => eprintln!("Error: {:?}", e),
    }
    //let test = fs::read_to_string("../bignumber.imp")?;
    //let result = ProgramAllParser::new().parse(&test);
    //println!("{:?}", result);
    /*
    let program1 = fs::read_to_string("../example1.imp")?;
    let result1 = ProgramAllParser::new().parse(&program1);
    println!("ex1: {:?}", result1);

    let program2 = fs::read_to_string("../example2.imp")?;
    let result2 = ProgramAllParser::new().parse(&program2);
    println!("ex2: {:?}", result2);

    let program3 = fs::read_to_string("../example3.imp")?;
    let result3 = ProgramAllParser::new().parse(&program3);
    println!("ex3: {:?}", result3);

    let program4 = fs::read_to_string("../example4.imp")?;
    let result4 = ProgramAllParser::new().parse(&program4);
    println!("ex4: {:?}", result4);

    let program5 = fs::read_to_string("../example5.imp")?;
    let result5 = ProgramAllParser::new().parse(&program5);
    println!("ex5: {:?}", result5);

    let program6 = fs::read_to_string("../example6.imp")?;
    let result6 = ProgramAllParser::new().parse(&program6);
    println!("ex6: {:?}", result6);

    let program7 = fs::read_to_string("../example7.imp")?;
    let result7 = ProgramAllParser::new().parse(&program7);
    println!("ex7: {:?}", result7);

    let program8 = fs::read_to_string("../example8.imp")?;
    let result8 = ProgramAllParser::new().parse(&program8);
    println!("ex8: {:?}", result8);

    let program9 = fs::read_to_string("../example9.imp")?;
    let result9 = ProgramAllParser::new().parse(&program9);
    println!("ex9: {:?}", result9);
    */

    Ok(())
}
