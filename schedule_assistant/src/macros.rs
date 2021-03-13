
pub use crate::json;

#[macro_export]
macro_rules! extract_or_continue {
    ($value:tt, $attribute:tt) => {
        match json::attribute_from_value($value, $attribute) {
            Some(value) => value,
            None => {
                println!("Unable to find {}, continuing", $attribute);
                continue;
            }
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

// macro_rules! opt_guard {
//     ($e:expr) => {
//         match $e {
//             Some(value) => value,
//             None => return Some(false),
//         }
//     };
// }

// macro_rules! err_guard {
//     ($e:expr) => {
//         match $e {
//             Some(value) => value,
//             None => return Ok(false),
//         }
//     };
// }