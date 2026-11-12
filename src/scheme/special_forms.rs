use std::collections::HashMap;
use std::sync::Arc;

use super::{apply, eval_expr, Errors, Expr, Frame, Procedure, Value};

pub fn try_special_form(
    name: &str,
    args: &[Expr],
    frame: &Arc<Frame>,
) -> Option<Result<Value, Errors>> {
    match name {
        "if" => Some(eval_if(args, frame)),
        "lambda" => Some(eval_lambda(args, frame)),
        "and" => Some(eval_and(args, frame)),
        "let" => Some(eval_let(args, frame)),
        "or" => Some(eval_or(args, frame)),
        "quote" => Some(eval_quote(args)),
        "cond" => Some(eval_cond(args, frame)),
        _ => None,
    }
}

fn eval_cond(args: &[Expr], frame: &Arc<Frame>) -> Result<Value, Errors> {
    for (i, clause) in args.iter().enumerate() {
        let Expr::List(parts) = clause else {
            return Err(Errors::MalformedCond);
        };

        let [test, body] = parts.as_slice() else {
            return Err(Errors::MalformedCond);
        };

        if matches!(test, Expr::Symbol(s) if s == "else") {
            if i != args.len() - 1 {
                return Err(Errors::MalformedCond);   // else must be last
            }
            return eval_expr(body, frame);
        }

        let result = eval_expr(test, frame)?;

        if !matches!(result, Value::Bool(false)) {
            return eval_expr(body, frame);
        }
    }

    Ok(Value::Bool(false))
}

fn eval_quote(args: &[Expr]) -> Result<Value, Errors> {
    let [datum] = args else {
        return Err(Errors::MalformedQuote);
    };

    quote_datum(datum)
}

fn quote_datum(expr: &Expr) -> Result<Value, Errors> {
    Ok(match expr {
        Expr::Integer(i) => Value::Integer(*i),
        Expr::Float(f) => Value::Float(*f),
        Expr::Str(s) => Value::Str(s.clone()),
        Expr::Bool(b) => Value::Bool(*b),
        Expr::List(items) => Value::List(
            items.iter().map(quote_datum).collect::<Result<Vec<_>, _>>()?,
        ),
        Expr::Symbol(_) => return Err(Errors::UnsupportedQuote),
        Expr::Char(c) => Value::Char(*c),
    })
}

fn eval_or(args: &[Expr], frame: &Arc<Frame>) -> Result<Value, Errors> {
    for expr in args {
        let result = eval_expr(expr, frame)?;

        if !matches!(result, Value::Bool(false)) {
            return Ok(result);
        }
    }

    Ok(Value::Bool(false))
}

fn eval_let(args: &[Expr], frame: &Arc<Frame>) -> Result<Value, Errors> {
    if let [Expr::Symbol(loop_name), Expr::List(bindings), body] = args {
        let mut params = Vec::with_capacity(bindings.len());
        let mut inits = Vec::with_capacity(bindings.len());

        for binding in bindings {
            let Expr::List(pair) = binding else {
                return Err(Errors::MalformedLet);
            };

            let [Expr::Symbol(param), init_expr] = pair.as_slice() else {
                return Err(Errors::MalformedLet);
            };

            params.push(param.clone());
            inits.push(eval_expr(init_expr, frame)?);   // outer frame
        }

        let procedure = Value::Procedure(Procedure {
            name: Some(loop_name.clone()),
            params,
            body: body.clone(),
            captured: Arc::clone(frame),
        });

        return apply(&procedure, inits);
    }

    let [binding_list, body] = args else {
        return Err(Errors::MalformedLet);
    };

    let Expr::List(binding_exprs) = binding_list else {
        return Err(Errors::MalformedLet);
    };

    let mut new_bindings = HashMap::new();

    for binding in binding_exprs {
        let Expr::List(pair) = binding else {
            return Err(Errors::MalformedLet);
        };

        let [Expr::Symbol(name), value_expr] = pair.as_slice() else {
            return Err(Errors::MalformedLet);
        };

        let value = eval_expr(value_expr, frame)?;
        new_bindings.insert(name.clone(), value);
    }

    let inner = Arc::new(Frame {
        bindings: new_bindings,
        parent: Some(Arc::clone(frame)),
    });

    eval_expr(body, &inner)
}

fn eval_and(args: &[Expr], frame: &Arc<Frame>) -> Result<Value, Errors> {
    let mut result = Value::Bool(true);

    for expr in args {
        result = eval_expr(expr, frame)?;

        if matches!(result, Value::Bool(false)) {
            return Ok(result);
        }
    }

    Ok(result)
}

fn eval_if(args: &[Expr], frame: &Arc<Frame>) -> Result<Value, Errors> {
    let [condition, consequent, alternative] = args else {
        return Err(Errors::MalformedIf);
    };

    let test = eval_expr(condition, frame)?;

    let branch = match test {
        Value::Bool(false) => alternative,
        _ => consequent,
    };

    eval_expr(branch, frame)
}

fn eval_lambda(args: &[Expr], frame: &Arc<Frame>) -> Result<Value, Errors> {
    let [params, body] = args else {
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

    Ok(Value::Procedure(Procedure {
        name: None,
        params,
        body: body.clone(),
        captured: Arc::clone(frame),
    }))
}
