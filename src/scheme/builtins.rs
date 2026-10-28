use super::Value;
use super::errors::Errors;

pub fn string_append(values: Vec<Value>) -> Result<Value, Errors> {
    let mut result = String::new();

    for value in &values {
        match value.as_str() {
            Some(s) => result.push_str(s),
            None => {
                return Err(Errors::TypeMismatch);
            }
        }
    }

    Ok(Value::Str(result))
}
