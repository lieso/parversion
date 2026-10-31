use std::collections::HashMap;
use std::sync::Arc;

mod builtins;
mod errors;
mod lexer;
mod parser;
mod special_forms;

use builtins::*;
pub use errors::Errors;
use lexer::tokenize;
use parser::{parse, Expr};
use special_forms::try_special_form;

#[derive(Clone, Debug)]
pub enum Value {
    Integer(i64),
    Float(f64),
    Str(String),
    Char(char),
    Bool(bool),
    Procedure(Procedure),
}

#[derive(Clone, Debug)]
pub struct Procedure {
    name: Option<String>,
    params: Vec<String>,
    body: Expr,
    captured: Arc<Frame>,
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

#[derive(Debug)]
struct Frame {
    bindings: HashMap<String, Value>,
    parent: Option<Arc<Frame>>,
}

impl Frame {
    fn new() -> Self {
        Frame {
            bindings: HashMap::new(),
            parent: None,
        }
    }

    fn get(&self, name: &str) -> Option<&Value> {
        self.bindings
            .get(name)
            .or_else(|| self.parent.as_ref()?.get(name))
    }
}

pub fn eval(source: &str) -> Result<Value, Errors> {
    let tokens = tokenize(source)?;
    let expression = parse(tokens)?;

    eval_expr(&expression, &Arc::new(Frame::new()))
}

pub fn apply(procedure: &Value, args: Vec<Value>) -> Result<Value, Errors> {
    let Value::Procedure(p) = procedure else {
        return Err(Errors::NotAProcedure);
    };

    let mut bindings = HashMap::new();

    if let Some(name) = &p.name {
        bindings.insert(name.clone(), procedure.clone());
    }

    for (name, value) in p.params.iter().zip(args) {
        bindings.insert(name.clone(), value);
    }

    let frame = Arc::new(Frame {
        bindings,
        parent: Some(Arc::clone(&p.captured)),
    });

    eval_expr(&p.body, &frame)
}

fn eval_expr(expr: &Expr, frame: &Arc<Frame>) -> Result<Value, Errors> {
    let value = match expr {
        Expr::Integer(i) => Value::Integer(*i),
        Expr::Float(f) => Value::Float(*f),
        Expr::Str(s) => Value::Str(s.clone()),
        Expr::Bool(b) => Value::Bool(*b),
        Expr::Symbol(q) => frame.get(q).cloned().ok_or_else(|| Errors::UnboundSymbol)?,
        Expr::List(l) => {
            let Some((first, rest)) = l.split_first() else {
                return Err(Errors::EmptyCall);
            };

            let Expr::Symbol(name) = first else {
                return Err(Errors::NotAProcedure);
            };

            if let Some(result) = try_special_form(name, rest, frame) {
                return result;
            }

            let values = rest
                .iter()
                .map(|e| eval_expr(e, frame))
                .collect::<Result<Vec<_>, _>>()?;

            if let Some(procedure @ Value::Procedure(_)) = frame.get(name) {
                return apply(procedure, values);
            }

            match name.as_str() {
                "string-append" => string_append(values)?,
                "string-length" => string_length(values)?,
                "string-ref" => string_ref(values)?,
                "substring" => substring(values)?,
                "string->number" => string_to_number(values)?,
                "char-whitespace?" => char_whitespace(values)?,
                "char-numeric?" => char_numeric(values)?,
                "<" => less_than(values)?,
                "+" => add(values)?,
                _ => return Err(Errors::UnboundSymbol),
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
        let procedure =
            eval(r#"(lambda (value) (string-append "https://news.ycombinator.com/" value))"#)
                .unwrap();

        let result = apply(&procedure, vec![Value::from("item?id=1")]).unwrap();

        assert_eq!(
            result.as_str(),
            Some("https://news.ycombinator.com/item?id=1")
        );
    }

    #[test]
    fn eval_if_chooses_branch() {
        assert_eq!(eval(r#"(if #t "yes" "no")"#).unwrap().as_str(), Some("yes"));
        assert_eq!(eval(r#"(if #f "yes" "no")"#).unwrap().as_str(), Some("no"));
        assert_eq!(eval(r#"(if 0 "yes" "no")"#).unwrap().as_str(), Some("yes"));
    }

    #[test]
    fn eval_if_skips_untaken_branch() {
        let value = eval(r#"(if #t 1 (string-ref "" 5))"#).unwrap();

        assert!(matches!(value, Value::Integer(1)));
    }

    // Only supporting two arguments to "if" statements
    // Unlikely this will be required
    #[test]
    fn eval_if_missing_branch() {
        let result = eval(r#"(if #t 1)"#);

        assert!(matches!(result, Err(Errors::MalformedIf)));
    }

    #[test]
    fn eval_and_empty_is_true() {
        let value = eval("(and)").unwrap();

        assert!(matches!(value, Value::Bool(true)));
    }

    #[test]
    fn eval_and_returns_last_value() {
        assert_eq!(eval(r#"(and 1 "x")"#).unwrap().as_str(), Some("x"));
    }

    #[test]
    fn eval_and_short_circuits_on_false() {
        let value = eval(r#"(and #f (string-ref "" 5))"#).unwrap();

        assert!(matches!(value, Value::Bool(false)));
    }

    #[test]
    fn eval_and_stops_at_first_false() {
        let value = eval(r#"(and 1 #f (string-ref "" 5))"#).unwrap();

        assert!(matches!(value, Value::Bool(false)));
    }

    #[test]
    fn eval_let_body_sees_enclosing_parameter() {
        let procedure = eval(r#"(lambda (v) (let ((y 1)) v))"#).unwrap();

        let result = apply(&procedure, vec![Value::from("a")]).unwrap();

        assert_eq!(result.as_str(), Some("a"));
    }

    #[test]
    fn eval_let_bindings_do_not_see_siblings() {
        let result = eval(r#"(let ((x 1) (y x)) y)"#);

        assert!(matches!(result, Err(Errors::UnboundSymbol)));
    }
}
