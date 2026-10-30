use super::{eval_expr, Errors, Expr, Frame, Procedure, Value};

pub fn try_special_form(name: &str, args: &[Expr], frame: &Frame) -> Option<Result<Value, Errors>> {
    match name {
        "if" => Some(eval_if(args, frame)),
        "lambda" => Some(eval_lambda(args, frame)),
        "and" => Some(eval_and(args, frame)),
        _ => None,
    }
}

fn eval_and(args: &[Expr], frame: &Frame) -> Result<Value, Errors> {
    let mut result = Value::Bool(true);

    for expr in args {
        result = eval_expr(expr, frame)?;

        if matches!(result, Value::Bool(false)) {
            return Ok(result);
        }
    }

    Ok(result)
}

fn eval_if(args: &[Expr], frame: &Frame) -> Result<Value, Errors> {
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

fn eval_lambda(args: &[Expr], frame: &Frame) -> Result<Value, Errors> {
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
        params,
        body: body.clone(),
    }))
}
