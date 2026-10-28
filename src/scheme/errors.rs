use std::fmt;

#[derive(Clone, Debug)]
pub enum Errors {
    UnterminatedString,
    UnexpectedCharacter(char),
    UnexpectedCloseParen,
    UnexpectedEndOfInput,
}

impl fmt::Display for Errors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Errors::UnterminatedString => write!(f, "unterminated string literal"),
            Errors::UnexpectedCharacter(character) => write!(f, "unexpected character '{}'", character),
            Errors::UnexpectedCloseParen => write!(f, "unexpected close paren"),
            Errors::UnexpectedEndOfInput => write!(f, "unexpected end of input"),
        }
    }
}
