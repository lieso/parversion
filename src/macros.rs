#[macro_export]
macro_rules! read_lock {
    ($data:expr) => {
        $data.read().unwrap()
    };
}

pub(crate) use read_lock;

#[macro_export]
macro_rules! write_lock {
    ($data:expr) => {
        $data.write().unwrap()
    };
}

pub(crate) use write_lock;

