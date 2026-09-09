pub mod client;
pub mod constants;
pub mod vault_path_resolver;

use loco_rs::Error;
use std::fmt::Display;

use crate::models::vault::VaultItemStatus;

pub trait ResultExt<T, E> {
    /// Wraps the error into `loco_rs::Error::wrap` with custom tracing msg
    fn to_loco_inspect(self, msg: impl AsRef<str>) -> loco_rs::Result<T>
    where
        E: std::error::Error + Send + Sync + 'static;

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
    fn to_loco_inspect(self, msg: impl AsRef<str>) -> loco_rs::Result<T>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        if let Err(e) = &self {
            tracing::error!(error=%e, "{}", msg.as_ref());
        }
        // map_err can directly take the function pointer
        self.map_err(Error::wrap)
    }

    fn to_loco_err(self) -> loco_rs::Result<T>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        if let Err(e) = &self {
            tracing::error!(error=%e);
        }
        // map_err can directly take the function pointer
        self.map_err(Error::wrap)
    }

    fn to_loco_string(self) -> loco_rs::Result<T>
    where
        E: Display,
    {
        if let Err(e) = &self {
            tracing::error!(error=%e);
        }
        self.map_err(|e| Error::Message(e.to_string()))
    }
}

/// Construct loco error message
/// ```
/// Error::Message(format!("{}", e))
/// ```
#[macro_export]
macro_rules! loco_err_msg {
    ($($arg:tt)*) => {
            loco_rs::Error::Message(format!($($arg)*))
        };
}

/// Construct error with loco message
/// ```
/// Err(Error::Message(format!("{}", e)))
/// ```
#[macro_export]
macro_rules! loco_err {
    ($($arg:tt)*) => {
            Err(loco_rs::Error::Message(format!($($arg)*)))
        };
}

/// Not in READY | CANCELLED | FAILED
pub fn is_active_status(status: &VaultItemStatus) -> bool {
    !matches!(
        status,
        VaultItemStatus::READY | VaultItemStatus::CANCELLED | VaultItemStatus::FAILED
    )
}
