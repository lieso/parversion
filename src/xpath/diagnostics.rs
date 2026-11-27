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

    pub fn log(&self, message: &str) {
        let mut logs = self.logs.lock().unwrap();
        logs.push_str(message);
        logs.push('\n');
    }

    pub fn take(&self) -> String {
        std::mem::take(&mut *self.logs.lock().unwrap())
    }
}
