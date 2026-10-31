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
        _ => None,
    }
}

fn eval_let(args: &[Expr], frame: &Arc<Frame>) -> Result<Value, Errors> {
    if let [Expr::Symbol(loop_name), Expr::List(bindings), body] = args {
        let [Expr::List(pair)] = bindings.as_slice() else {
            return Err(Errors::MalformedLet);
        };

        let [Expr::Symbol(param), init] = pair.as_slice() else {
            return Err(Errors::MalformedLet);
        };

        let init = eval_expr(init, frame)?;

        let procedure = Value::Procedure(Procedure {
            name: Some(loop_name.clone()),
            params: vec![param.clone()],
            body: body.clone(),
            captured: Arc::clone(frame),
        });

        return apply(&procedure, vec![init]);
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
