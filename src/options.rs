#[derive(Clone, Debug)]
pub struct Options {
    pub origin: Option<String>,
    pub date: Option<String>,
    pub regenerate: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            origin: None,
            date: None,
            regenerate: false,
        }
    }
}
