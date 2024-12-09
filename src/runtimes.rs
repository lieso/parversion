use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};
use serde_json::{Value, json};
use regex::Regex;

pub fn python(script: String, json_string: String) -> Result<String, String> {
    log::trace!("In python");

    Python::with_gil(|py| {
        let data: Value = serde_json::from_str(json_string).expect("Invalid JSON");
        let py_data = PyDict::new(py);
        if let JsonValue::Object(map) = data {
            for (key, value) in map {
                py_data.set_item(key, value.to_string()).unwrap();
            }
        }


    });

    unimplemented!()
}

pub fn awk(expression: String, input_data: String) -> Result<String, String> {
    log::trace!("In awk");

    let awk_expression = sanitize_awk_expression(&expression)?;

    let mut process = Command::new("awk")
        .arg(awk_expression)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn awk process");

    if let Some(mut stdin) = process.stdin.take() {
        stdin.write_all(input_data.as_bytes()).expect("Failed to write to stdin");
    }

    let output = process
        .wait_with_output()
        .expect("Failed to read awk output");

    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout);

        log::info!("Successfully evaluated awk expression with result: {}", result);

        Ok(result.to_string())
    } else {
        Err("Failed to evaluate awk expression")
    }
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
