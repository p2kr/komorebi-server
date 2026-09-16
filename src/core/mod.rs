pub mod client;
pub mod constants;
pub mod macros;
pub mod vault_path_resolver;

use cached::cached;
use loco_rs::Error;
use std::{
    fmt::Display,
    path::Path,
    time::{Duration, Instant},
};

use crate::dtos::VaultStatus;

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

    fn log_err(self) -> Self
    where
        E: Display;

    fn log_err_custom(self, msg: impl AsRef<str>) -> Self
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

    fn log_err(self) -> Self
    where
        E: Display,
    {
        if let Err(e) = &self {
            tracing::error!(error=%e);
        }
        self
    }

    fn log_err_custom(self, msg: impl AsRef<str>) -> Self
    where
        E: Display,
    {
        if let Err(e) = &self {
            tracing::error!(error=%e, "{}", msg.as_ref());
        }
        self
    }
}

/// Not in READY | CANCELLED | FAILED
pub fn is_active_status(status: &VaultStatus) -> bool {
    !matches!(
        status,
        VaultStatus::READY | VaultStatus::CANCELLED | VaultStatus::FAILED
    )
}

/// Sanitize the filename to prevent weird characters or path traversal
#[cached(max_size = 100)]
pub fn sanitize_filename(name: &str) -> String {
    // 1. Extract just the filename (drops malicious paths like "../../font.ttf")
    let base_name = Path::new(name)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(name);

    // 2. Replace invalid characters with underscores, drop unprintable control characters
    let mut safe_name = String::with_capacity(base_name.len());
    for c in base_name.chars() {
        match c {
            '/' | '\\' | '<' | '>' | ':' | '"' | '|' | '?' | '*' | '\0' => safe_name.push('_'),
            c if !c.is_control() => safe_name.push(c),
            _ => (), // Completely drop unprintable control characters
        }
    }

    // 3. Strip leading/trailing whitespaces and trailing dots (Windows OS protection)
    safe_name = safe_name.trim().trim_end_matches('.').to_string();

    // 4. Fallback for completely invalid inputs (e.g., "", "...", or "   ")
    if safe_name.is_empty() || safe_name.replace('.', "").is_empty() {
        safe_name = "unnamed_file".to_string();
    }

    // 5. Prevent Windows reserved names (CON, PRN, AUX, NUL, COM1-9, LPT1-9)
    let stem_upper = safe_name
        .split('.')
        .next()
        .unwrap_or("")
        .to_ascii_uppercase();
    let reserved = ["CON", "PRN", "AUX", "NUL"];
    if reserved.contains(&stem_upper.as_str())
        || (stem_upper.len() == 4
            && (stem_upper.starts_with("COM") || stem_upper.starts_with("LPT"))
            && stem_upper.chars().last().unwrap().is_ascii_digit())
    {
        safe_name.insert(0, '_'); // "CON.ttf" -> "_CON.ttf"
    }

    // 6. Smart Truncation (255 bytes limit) - Preserves the file extension
    if safe_name.len() > 255 {
        let (stem, ext) = match safe_name.rsplit_once('.') {
            // Only preserve the extension if it's reasonably short (e.g., <= 15 bytes)
            Some((s, e)) if !e.is_empty() && e.len() <= 15 => (s, Some(e)),
            _ => (safe_name.as_str(), None),
        };

        let ext_bytes = ext.map(|e| e.len() + 1).unwrap_or(0); // +1 accounts for the '.'
        let max_stem_bytes = 255usize.saturating_sub(ext_bytes);

        // Find nearest safe Unicode boundary to slice at
        let mut end = max_stem_bytes;
        while end > 0 && !stem.is_char_boundary(end) {
            end -= 1;
        }

        // Reassemble the truncated stem with the original extension
        safe_name = if let Some(e) = ext {
            format!("{}.{}", &stem[..end], e)
        } else {
            stem[..end].to_string()
        };
    }

    safe_name
}

pub struct Ticker {
    interval: Duration,
    last_tick: Option<Instant>,
}

impl Ticker {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            last_tick: None,
        }
    }

    pub fn tick(&mut self) -> bool {
        let now = Instant::now();
        if self
            .last_tick
            .is_none_or(|last| now.duration_since(last) >= self.interval)
        {
            self.last_tick = Some(now);
            true
        } else {
            false
        }
    }
}
