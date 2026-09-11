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
