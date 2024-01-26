use std::fs;
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

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: /path/to/programme <input-file> <output-file>");
        std::process::exit(1);
    }

    // read the input file

    let program = match fs::read_to_string(&args[1]) {
        Ok(contents) => contents,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }    
    };

    // parse the file

    match ProgramAllParser::new().parse(&program) {
        Ok(ast) => {

            // compile the program into mv code

            //println!("Parsing succeeded!\nAST: {:?}", ast);
            match translate(ast) {
                Ok(code) => {
                    let all_code = code
                    .iter()
                    .fold(Vec::new(), |mut all_code, line| {
                        writeln!(&mut all_code, "{}", line).unwrap();
                        all_code
                    });
                    if let Err(e) = fs::write(&args[2], all_code) {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    };
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
        Err(e) => {
            eprintln!("Error: {:?}", e);
            std::process::exit(1);
        }    
    }
}
