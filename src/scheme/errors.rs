use std::fmt;

#[derive(Clone, Debug)]
pub enum Errors {
    UnterminatedString,
    UnexpectedCharacter(char),
    UnexpectedCloseParen,
    UnexpectedEndOfInput,
    EmptyCall,
    NotAProcedure,
    TypeMismatch,
    UnboundSymbol,
    MalformedLambda,
    ArityMismatch{ expected: usize, got: usize },
    IndexOutOfRange,
    Overflow,
}

impl fmt::Display for Errors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Errors::UnterminatedString => write!(f, "unterminated string literal"),
            Errors::UnexpectedCharacter(character) => write!(f, "unexpected character '{}'", character),
            Errors::UnexpectedCloseParen => write!(f, "unexpected close paren"),
            Errors::UnexpectedEndOfInput => write!(f, "unexpected end of input"),
            Errors::EmptyCall => write!(f, "empty call"),
            Errors::TypeMismatch => write!(f, "type mismatch"),
            Errors::NotAProcedure => write!(f, "not a procedure"),
            Errors::UnboundSymbol => write!(f, "unbound symbol"),
            Errors::MalformedLambda => write!(f, "malformed lambda"),
            Errors::ArityMismatch { expected, got } => write!(f, "expected {} arguments, got {}", expected, got),
            Errors::IndexOutOfRange => write!(f, "index out of range"),
            Errors::Overflow => write!(f, "integer overflow"),
        }
    }
}
