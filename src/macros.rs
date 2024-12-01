#[macro_export]
macro_rules! read_lock {
    ($data:expr) => {
        $data.read().unwrap()
    };
}

#[macro_export]
macro_rules! write_lock {
    ($data:expr) => {
        $data.write().unwrap()
    };
}
