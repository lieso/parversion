use std::cmp::Ordering;

use super::Value;
use super::errors::Errors;

pub fn string_append(values: Vec<Value>) -> Result<Value, Errors> {
    let mut result = String::new();

    for value in &values {
        result.push_str(expect_str(value)?);
    }

    Ok(Value::Str(result))
}

pub fn string_length(values: Vec<Value>) -> Result<Value, Errors> {
    expect_arity(&values, 1)?;
    let s = expect_str(&values[0])?;

    Ok(Value::Integer(s.chars().count() as i64))
}

pub fn string_ref(values: Vec<Value>) -> Result<Value, Errors> {
    expect_arity(&values, 2)?;
    let s = expect_str(&values[0])?;
    let index = expect_index(&values[1])?;

    let c = s.chars().nth(index).ok_or(Errors::IndexOutOfRange)?;

    Ok(Value::Char(c))
}

pub fn substring(values: Vec<Value>) -> Result<Value, Errors> {
    expect_arity(&values, 3)?;
    let chars: Vec<char> = expect_str(&values[0])?.chars().collect();
    let start = expect_index(&values[1])?;
    let end = expect_index(&values[2])?;

    if start > end || end > chars.len() {
        return Err(Errors::IndexOutOfRange);
    }

    Ok(Value::Str(chars[start..end].iter().collect()))
}

pub fn string_to_number(values: Vec<Value>) -> Result<Value, Errors> {
    expect_arity(&values, 1)?;
    let s = expect_str(&values[0])?;

    if let Ok(i) = s.parse::<i64>() {
        return Ok(Value::Integer(i));
    }

    // Same digit guard as the lexer, so "inf" and "nan" are not numbers.
    if s.chars().any(|c| c.is_ascii_digit()) {
        if let Ok(f) = s.parse::<f64>() {
            return Ok(Value::Float(f));
        }
    }

    // R7RS: a string that isn't a number gives #f, not an error.
    Ok(Value::Bool(false))
}

pub fn char_whitespace(values: Vec<Value>) -> Result<Value, Errors> {
    expect_arity(&values, 1)?;

    Ok(Value::Bool(expect_char(&values[0])?.is_whitespace()))
}

pub fn char_numeric(values: Vec<Value>) -> Result<Value, Errors> {
    expect_arity(&values, 1)?;

    Ok(Value::Bool(expect_char(&values[0])?.is_ascii_digit()))
}

pub fn add(values: Vec<Value>) -> Result<Value, Errors> {
    let mut total = Value::Integer(0);

    for value in &values {
        total = add_two(&total, value)?;
    }

    Ok(total)
}

pub fn less_than(values: Vec<Value>) -> Result<Value, Errors> {
    for value in &values {
        as_float(value)?; // every argument must be a number
    }

    for pair in values.windows(2) {
        if compare(&pair[0], &pair[1])? != Ordering::Less {
            return Ok(Value::Bool(false));
        }
    }

    Ok(Value::Bool(true))
}

fn add_two(a: &Value, b: &Value) -> Result<Value, Errors> {
    match (a, b) {
        (Value::Integer(x), Value::Integer(y)) => {
            x.checked_add(*y).map(Value::Integer).ok_or(Errors::Overflow)
        }
        _ => Ok(Value::Float(as_float(a)? + as_float(b)?)),
    }
}

fn compare(a: &Value, b: &Value) -> Result<Ordering, Errors> {
    match (a, b) {
        (Value::Integer(x), Value::Integer(y)) => Ok(x.cmp(y)),
        _ => Ok(as_float(a)?.total_cmp(&as_float(b)?)),
    }
}

fn expect_arity(values: &[Value], expected: usize) -> Result<(), Errors> {
    if values.len() == expected {
        Ok(())
    } else {
        Err(Errors::ArityMismatch { expected, got: values.len() })
    }
}

fn expect_str(value: &Value) -> Result<&str, Errors> {
    value.as_str().ok_or(Errors::TypeMismatch)
}

fn expect_char(value: &Value) -> Result<char, Errors> {
    match value {
        Value::Char(c) => Ok(*c),
        _ => Err(Errors::TypeMismatch),
    }
}

fn expect_int(value: &Value) -> Result<i64, Errors> {
    match value {
        Value::Integer(i) => Ok(*i),
        _ => Err(Errors::TypeMismatch),
    }
}

fn expect_index(value: &Value) -> Result<usize, Errors> {
    let i = expect_int(value)?;

    usize::try_from(i).map_err(|_| Errors::IndexOutOfRange)
}

fn as_float(value: &Value) -> Result<f64, Errors> {
    match value {
        Value::Integer(i) => Ok(*i as f64),
        Value::Float(f) => Ok(*f),
        _ => Err(Errors::TypeMismatch),
    }
}
