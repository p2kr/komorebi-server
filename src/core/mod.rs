pub mod client;
pub mod constants;
pub mod vault_path_resolver;

use loco_rs::Error;
use std::fmt::Display;

pub trait ResultExt<T, E> {
    /// Wraps the error into `loco_rs::Error::wrap`
    fn to_loco_err(self) -> loco_rs::Result<T>
    where
        E: std::error::Error + Send + Sync + 'static;

    /// Converts the error to a string and wraps it in `loco_rs::Error::Message`
    fn to_loco_string(self) -> loco_rs::Result<T>
    where
        E: Display;
}

impl<T, E> ResultExt<T, E> for Result<T, E> {
    fn to_loco_err(self) -> loco_rs::Result<T>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        // map_err can directly take the function pointer
        self.map_err(Error::wrap)
    }

    fn to_loco_string(self) -> loco_rs::Result<T>
    where
        E: Display,
    {
        self.map_err(|e| Error::Message(e.to_string()))
    }
}

/// Construct error with loco message
/// ```
/// Err(Error::Message(format!("{}", e)))
/// ```
#[macro_export]
macro_rules! loco_err {
    ($($arg:tt)*) => {
            Err(loco_err_msg!($($arg)*))
        };
}

/// Construct loco error message
/// ```
/// Error::Message(format!("{}", e))
/// ```
#[macro_export]
macro_rules! loco_err_msg {
    ($($arg:tt)*) => {
            Error::Message(format!($($arg)*))
        };
}
