mod errors;
mod lexer;
mod parser;
mod builtins;

pub use errors::Errors;
use lexer::tokenize;
use parser::{Expr, parse};
use builtins::*;

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
    let tokens = tokenize(source)?;
    let expression = parse(tokens)?;

    eval_expr(&expression)
}

pub fn apply(procedure: &Value, args: Vec<Value>) -> Result<Value, Errors> {
    let _ = (procedure, args);
    unimplemented!()
}

fn eval_expr(expr: &Expr) -> Result<Value, Errors> {
    let value = match expr {
        Expr::Integer(i) => Value::Integer(*i),
        Expr::Float(f) => Value::Float(*f),
        Expr::Str(s) => Value::Str(s.clone()),
        Expr::Bool(b) => Value::Bool(*b),
        Expr::Symbol(q) => unimplemented!(),
        Expr::List(l) => {
            let Some((first, rest)) = l.split_first() else {
                return Err(Errors::EmptyCall);
            };

            let Expr::Symbol(name) = first else {
                return Err(Errors::NotAProcedure);
            };

            let values = rest.iter().map(eval_expr).collect::<Result<Vec<_>, _>>()?;

            match name.as_str() {
                "string-append" => string_append(values)?,
                _ => unimplemented!(),
            }
        }
    };

    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_integer_literal() {
        let value = eval("42").unwrap();

        assert!(matches!(value, Value::Integer(42)));
    }

    #[test]
    fn eval_string_append() {
        let value = eval(r#"(string-append "https://example.com" "/items/123")"#).unwrap();

        assert_eq!(value.as_str(), Some("https://example.com/items/123"));
    }
}
