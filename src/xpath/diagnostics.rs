use std::sync::Mutex;

use crate::prelude::*;

pub struct Diagnostics {
    pub logs: Mutex<String>,
}

impl Diagnostics {
    pub fn new() -> Self {
        Diagnostics {
            logs: Mutex::new(String::new()),
        }
    }
}
