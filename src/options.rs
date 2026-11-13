#[derive(Clone, Debug)]
pub struct Options {
    pub regenerate: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            regenerate: false,
        }
    }
}
