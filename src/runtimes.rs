use regex::Regex;
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::io::Write;
use serde_json;
use pyo3::types::IntoPyDict;

pub fn python_field_constant(
    code: &str,
    input_map: HashMap<String, String>
) -> Result<HashMap<String, String>, String> {

    unimplemented!()

    //Python::with_gil(|py| {
    //    let input_json = serde_json::to_string(&input_map).expect("Failed to serialize HashMap");

    //    //import json

    //    //def process_json_string(json_string):
    //    //    input_dict = json.loads(json_string)

    //    //    filtered_dict = {k: v for k, v in input_dict.items() if 'keep' in v}

    //    //    return json.dumps(filtered_dict)

    //    unimplemented!()
    //})
}

pub fn python_field_map(code: &str, input_map: HashMap<String, String>) -> Result<HashMap<String, String>, String> {

    unimplemented!()

    //Python::with_gil(|py| {
    //    let py_dict = PyDict::new(py);
    //    for (key, value) in &input_map {
    //        py_dict.set_item(key, value)?;
    //    }

    //    // keys_of_interest = {'href', 'title'}
    //    // output = {k: v for k, v in input_dict.items() if k in keys_of_interest}

    //    let globals = [("input_dict", py_dict)].into_py_dict(py);
    //    py.run(py_code, Some(globals), None)?;

    //    let filtered_py_dict: &PyDict = globals.get_item("output").unwrap().downcast().unwrap();

    //    let mut output_map = HashMap::new();
    //    for (key, value) in filtered_py_dict {
    //        output_map.insert(key.extract::<String>()?, value.extract::<String>()?);
    //    }

    //    Ok(output_map)
    //})
}

pub fn awk(expression: String, input_data: String) -> Result<String, String> {
    log::trace!("In awk");

    unimplemented!()

    //let awk_expression = sanitize_awk_expression(&expression)?;

    //let mut process = Command::new("awk")
    //    .arg(awk_expression)
    //    .stdin(Stdio::piped())
    //    .stdout(Stdio::piped())
    //    .spawn()
    //    .expect("Failed to spawn awk process");

    //if let Some(mut stdin) = process.stdin.take() {
    //    stdin.write_all(input_data.as_bytes()).expect("Failed to write to stdin");
    //}

    //let output = process
    //    .wait_with_output()
    //    .expect("Failed to read awk output");

    //if output.status.success() {
    //    let result = String::from_utf8_lossy(&output.stdout);

    //    log::info!("Successfully evaluated awk expression with result: {}", result);

    //    Ok(result.to_string())
    //} else {
    //    Err("Failed to evaluate awk expression".to_string())
    //}
}

pub fn sanitize_awk_expression(input: &str) -> Result<String, String> {
    let re = Regex::new(r"^awk\s*'([^']*)'$").map_err(|e| e.to_string())?;

    re.captures(input).and_then(|caps| {
        caps.get(1).map(|matched_text| matched_text.as_str().to_string())
    })

    if let Some(caps) = re.captures(input) {
        if let Some(matched_text) = caps.get(1) {
            return Ok(matched_text.as_str().to_string());
        }
    }

    Err("Input did not match the expected awk pattern".to_string())
}
