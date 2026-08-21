pub mod constants;

use std::{fmt::Display, result::Result};

use loco_rs::Error;

pub trait ResultExt<T> {
    fn to_loco_err(self) -> loco_rs::Result<T>;
}

impl<T, E: Display> ResultExt<T> for Result<T, E> {
    fn to_loco_err(self) -> loco_rs::Result<T> {
        self.map_err(|e| Error::Message(e.to_string()))
    }
}
