mod errors;
mod lexer;

pub use errors::Errors;
use lexer::tokenize;

#[derive(Clone, Debug)]
pub enum Value {
    Integer(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Procedure(Procedure),
}

#[derive(Clone, Debug)]
pub struct Procedure {

}

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::Str(s.to_string())
    }
}

struct Frame {

}

pub fn eval(source: &str) -> Result<Value, Errors> {
    let tokens = tokenize(source);
    unimplemented!()
}

pub fn apply(procedure: &Value, args: Vec<Value>) -> Result<Value, Errors> {
    let _ = (procedure, args);
    unimplemented!()
}
