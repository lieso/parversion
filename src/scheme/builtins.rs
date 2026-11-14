use std::cmp::Ordering;

use super::errors::Errors;
use super::Value;

pub fn integer_to_char(values: Vec<Value>) -> Result<Value, Errors> {
    expect_arity(&values, 1)?;
    let code = expect_index(&values[0])?;
    let c = u32::try_from(code)
        .ok()
        .and_then(char::from_u32)
        .ok_or(Errors::IndexOutOfRange)?;

    Ok(Value::Char(c))
}

pub fn number_p(values: Vec<Value>) -> Result<Value, Errors> {
    expect_arity(&values, 1)?;

    Ok(Value::Bool(matches!(
        values[0],
        Value::Integer(_) | Value::Float(_)
    )))
}

pub fn string_prefix(values: Vec<Value>) -> Result<Value, Errors> {
    expect_arity(&values, 2)?;
    let prefix = expect_str(&values[0])?;
    let string = expect_str(&values[1])?;

    Ok(Value::Bool(string.starts_with(prefix)))
}

pub fn char_alphabetic(values: Vec<Value>) -> Result<Value, Errors> {
    expect_arity(&values, 1)?;

    Ok(Value::Bool(expect_char(&values[0])?.is_alphabetic()))
}

pub fn integer_p(values: Vec<Value>) -> Result<Value, Errors> {
    expect_arity(&values, 1)?;

    // (integer? x) is #f for non-numbers, not an error, as in standard Scheme.
    Ok(Value::Bool(match &values[0] {
        Value::Integer(_) => true,
        Value::Float(f) => f.is_finite() && f.fract() == 0.0,
        _ => false,
    }))
}

pub fn multiply(values: Vec<Value>) -> Result<Value, Errors> {
    let mut total = Value::Integer(1);

    for value in &values {
        total = mul_two(&total, value)?;
    }

    Ok(total)
}

fn mul_two(a: &Value, b: &Value) -> Result<Value, Errors> {
    match (a, b) {
        (Value::Integer(x), Value::Integer(y)) => x
            .checked_mul(*y)
            .map(Value::Integer)
            .ok_or(Errors::Overflow),
        _ => Ok(Value::Float(as_float(a)? * as_float(b)?)),
    }
}

fn chain_compare_chars(
    values: &[Value],
    accept: fn(Ordering) -> bool,
) -> Result<Value, Errors> {
    let chars = values
        .iter()
        .map(expect_char)
        .collect::<Result<Vec<_>, _>>()?; // type-check every argument first

    Ok(Value::Bool(
        chars.windows(2).all(|p| accept(p[0].cmp(&p[1]))),
    ))
}

pub fn char_less_equal(values: Vec<Value>) -> Result<Value, Errors> {
    chain_compare_chars(&values, |o| o != Ordering::Greater)
}

pub fn char_greater_equal(values: Vec<Value>) -> Result<Value, Errors> {
    chain_compare_chars(&values, |o| o != Ordering::Less)
}

pub fn string_equals(values: Vec<Value>) -> Result<Value, Errors> {
    let strs = values
        .iter()
        .map(expect_str)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Value::Bool(strs.windows(2).all(|p| p[0] == p[1])))
}

pub fn number_to_string(values: Vec<Value>) -> Result<Value, Errors> {
    expect_arity(&values, 1)?;

    match &values[0] {
        Value::Integer(i) => Ok(Value::Str(i.to_string())),
        Value::Float(f) => Ok(Value::Str(f.to_string())),
        _ => Err(Errors::TypeMismatch),
    }
}

fn chain_compare(values: &[Value], accept: fn(Ordering) -> bool) -> Result<Value, Errors> {
    for value in values {
        as_float(value)?; // every argument must be a number
    }

    for pair in values.windows(2) {
        if !accept(compare(&pair[0], &pair[1])?) {
            return Ok(Value::Bool(false));
        }
    }

    Ok(Value::Bool(true))
}

pub fn greater_than(values: Vec<Value>) -> Result<Value, Errors> {
    chain_compare(&values, |o| o == Ordering::Greater)
}

pub fn less_equal(values: Vec<Value>) -> Result<Value, Errors> {
    chain_compare(&values, |o| o != Ordering::Greater)
}

pub fn greater_equal(values: Vec<Value>) -> Result<Value, Errors> {
    chain_compare(&values, |o| o != Ordering::Less)
}

pub fn char_equals(values: Vec<Value>) -> Result<Value, Errors> {
    let mut chars = Vec::with_capacity(values.len());
    for value in &values {
        chars.push(expect_char(value)?); // type-check every argument first
    }

    Ok(Value::Bool(chars.windows(2).all(|pair| pair[0] == pair[1])))
}

pub fn pair_p(values: Vec<Value>) -> Result<Value, Errors> {
    expect_arity(&values, 1)?;

    Ok(Value::Bool(
        matches!(&values[0], Value::List(l) if !l.is_empty()),
    ))
}

pub fn subtract(values: Vec<Value>) -> Result<Value, Errors> {
    let Some((first, rest)) = values.split_first() else {
        return Err(Errors::ArityMismatch {
            expected: 1,
            got: 0,
        });
    };

    // (- x) negates
    if rest.is_empty() {
        return sub_two(&Value::Integer(0), first);
    }

    let mut total = first.clone();
    for value in rest {
        total = sub_two(&total, value)?;
    }

    Ok(total)
}

fn sub_two(a: &Value, b: &Value) -> Result<Value, Errors> {
    match (a, b) {
        (Value::Integer(x), Value::Integer(y)) => x
            .checked_sub(*y)
            .map(Value::Integer)
            .ok_or(Errors::Overflow),
        _ => Ok(Value::Float(as_float(a)? - as_float(b)?)),
    }
}

pub fn string_foldcase(values: Vec<Value>) -> Result<Value, Errors> {
    expect_arity(&values, 1)?;

    Ok(Value::Str(expect_str(&values[0])?.to_lowercase()))
}

pub fn null_p(values: Vec<Value>) -> Result<Value, Errors> {
    expect_arity(&values, 1)?;

    // (null? x) is #f for non-lists, not an error, as in standard Scheme.
    Ok(Value::Bool(
        matches!(&values[0], Value::List(l) if l.is_empty()),
    ))
}

pub fn car(values: Vec<Value>) -> Result<Value, Errors> {
    expect_arity(&values, 1)?;
    let items = expect_list(&values[0])?;

    items.first().cloned().ok_or(Errors::IndexOutOfRange)
}

pub fn cdr(values: Vec<Value>) -> Result<Value, Errors> {
    expect_arity(&values, 1)?;
    let items = expect_list(&values[0])?;

    if items.is_empty() {
        return Err(Errors::IndexOutOfRange);
    }

    Ok(Value::List(items[1..].to_vec()))
}

pub fn cons(values: Vec<Value>) -> Result<Value, Errors> {
    expect_arity(&values, 2)?;
    let tail = expect_list(&values[1])?;

    let mut items = Vec::with_capacity(tail.len() + 1);
    items.push(values[0].clone());
    items.extend(tail.iter().cloned());

    Ok(Value::List(items))
}

pub fn reverse(values: Vec<Value>) -> Result<Value, Errors> {
    expect_arity(&values, 1)?;
    let items = expect_list(&values[0])?;

    Ok(Value::List(items.iter().rev().cloned().collect()))
}

pub fn string_to_list(values: Vec<Value>) -> Result<Value, Errors> {
    expect_arity(&values, 1)?;
    let s = expect_str(&values[0])?;

    Ok(Value::List(s.chars().map(Value::Char).collect()))
}

pub fn list_to_string(values: Vec<Value>) -> Result<Value, Errors> {
    expect_arity(&values, 1)?;
    let items = expect_list(&values[0])?;

    let mut result = String::new();
    for item in items {
        result.push(expect_char(item)?);
    }

    Ok(Value::Str(result))
}

pub fn equals(values: Vec<Value>) -> Result<Value, Errors> {
    for pair in values.windows(2) {
        let equal = match (&pair[0], &pair[1]) {
            (Value::Integer(x), Value::Integer(y)) => x == y,
            (a, b) => as_float(a)? == as_float(b)?,
        };

        if !equal {
            return Ok(Value::Bool(false));
        }
    }

    Ok(Value::Bool(true))
}

pub fn not(values: Vec<Value>) -> Result<Value, Errors> {
    expect_arity(&values, 1)?;

    Ok(Value::Bool(matches!(values[0], Value::Bool(false))))
}

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
        (Value::Integer(x), Value::Integer(y)) => x
            .checked_add(*y)
            .map(Value::Integer)
            .ok_or(Errors::Overflow),
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
        Err(Errors::ArityMismatch {
            expected,
            got: values.len(),
        })
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

fn expect_list(value: &Value) -> Result<&Vec<Value>, Errors> {
    match value {
        Value::List(items) => Ok(items),
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
