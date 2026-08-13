/// Extracts a substring using Python-style slicing semantics.
///
/// - Negative indices count from the end of the string (e.g., `-5` means 5 chars from the end).
/// - An `end` of `0` means "to the end of the string" (equivalent to omitting the slice end in Python).
/// - Out-of-bounds indices are clamped to the string length.
///
/// # Examples
///
/// ```
/// use komorebi_server::core::utils::substring;
///
/// assert_eq!(substring("hello world", 0, 5), "hello");
/// assert_eq!(substring("hello world", -5, 0), "world");
/// assert_eq!(substring("hello world", 6, -1), "worl");
/// ```
pub fn substring(s: &str, start: i64, end: i64) -> String {
    let len = s.chars().count() as i64;

    // Resolve negative indices relative to string length
    let resolve = |idx: i64| -> usize {
        if idx < 0 {
            (len + idx).max(0) as usize
        } else {
            idx.min(len) as usize
        }
    };

    let start_idx = resolve(start);
    // An end of 0 means "to the end" (like omitting the end in Python slicing)
    let end_idx = if end == 0 { len as usize } else { resolve(end) };

    if start_idx >= end_idx {
        return String::new();
    }

    s.chars().skip(start_idx).take(end_idx - start_idx).collect()
}
