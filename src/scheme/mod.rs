use std::collections::HashMap;

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
    params: Vec<String>,
    body: Expr, 
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
    bindings: HashMap<String, Value>,
}

impl Frame {
    fn new() -> Self {
        Frame { bindings: HashMap::new() }
    }

    fn get(&self, name: &str) -> Option<&Value> {
        self.bindings.get(name)
    }
}

pub fn eval(source: &str) -> Result<Value, Errors> {
    let tokens = tokenize(source)?;
    let expression = parse(tokens)?;

    eval_expr(&expression, &Frame::new())
}

pub fn apply(procedure: &Value, args: Vec<Value>) -> Result<Value, Errors> {
    let Value::Procedure(p) = procedure else {
        return Err(Errors::NotAProcedure);
    };

    let mut frame = Frame::new();
    for (name, value) in p.params.iter().zip(args) {
        frame.bindings.insert(name.clone(), value);
    }

    eval_expr(&p.body, &frame)
}

fn eval_expr(expr: &Expr, frame: &Frame) -> Result<Value, Errors> {
    let value = match expr {
        Expr::Integer(i) => Value::Integer(*i),
        Expr::Float(f) => Value::Float(*f),
        Expr::Str(s) => Value::Str(s.clone()),
        Expr::Bool(b) => Value::Bool(*b),
        Expr::Symbol(q) => frame
            .get(q)
            .cloned()
            .ok_or_else(|| Errors::UnboundSymbol)?,
        Expr::List(l) => {
            let Some((first, rest)) = l.split_first() else {
                return Err(Errors::EmptyCall);
            };

            let Expr::Symbol(name) = first else {
                return Err(Errors::NotAProcedure);
            };

            if name == "lambda" {
                let [params, body] = rest else {
                    return Err(Errors::MalformedLambda);
                };

                let Expr::List(param_exprs) = params else {
                    return Err(Errors::MalformedLambda);
                };

                let params = param_exprs
                    .iter()
                    .map(|p| match p {
                        Expr::Symbol(s) => Ok(s.clone()),
                        _ => Err(Errors::MalformedLambda),
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                return Ok(Value::Procedure(Procedure { params, body: body.clone() }));
            }

            let values = rest.iter().map(|e| eval_expr(e, frame)).collect::<Result<Vec<_>, _>>()?;

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

    #[test]
    fn apply_lambda() {
        let procedure = eval(
            r#"(lambda (value) (string-append "https://news.ycombinator.com/" value))"#
        ).unwrap();

        let result = apply(&procedure, vec![Value::from("item?id=1")]).unwrap();

        assert_eq!(result.as_str(), Some("https://news.ycombinator.com/item?id=1"));
    }
}
