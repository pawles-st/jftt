use std::fmt;

#[derive(Debug)]
pub enum ImpError {
    NumberTooBig(String),
}

impl fmt::Display for ImpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ImpError::NumberTooBig(number) = self;
        write!(f, "Number {} is too big", number)
    }
}
