use std::io;
use rcalc::modulo::P;

// lalrpop

use lalrpop_util::lalrpop_mod;
use grammar::ExprParser;

lalrpop_mod!(pub grammar);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CalculatorError {
    NumberExceededRange,
}

fn main() -> io::Result<()> {
    let mut buf = String::new();
    let handle = io::stdin();
    loop {
        let bytes_read = handle.read_line(&mut buf)?;
        if bytes_read == 0 {
            break;
        } else {
            let result = ExprParser::new().parse(&P, &buf.trim_end_matches('\n'));
            println!("{:?}", result);
            buf.clear();
        }
    }
    
    Ok(())
}
