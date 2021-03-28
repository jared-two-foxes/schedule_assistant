
pub use crate::json;

#[macro_export]
macro_rules! match_or_continue {
    ($expression:expr) => {
        match $expression {
            Some(value) => value,
            None => continue
        };
    };
}

#[macro_export]
macro_rules! guard {
    ($e:expr) => {
        match $e {
            Some(value) => value,
            None => return false,
        }
    };
}