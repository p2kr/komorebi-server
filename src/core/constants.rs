use std::{env, sync::LazyLock};

pub const DEFAULT_HOSTED_AUTH_PAGE: &str = "https://p2kr.github.io/komorebi-web/auth.html";
pub static VAULT_LOC: LazyLock<String> =
    LazyLock::new(|| env::var("VAULT_LOC").unwrap_or("vault".into()));
pub static ENCODED_LOC: LazyLock<String> = LazyLock::new(|| {
    env::var("ENCODED_LOC")
        .or_else(|_| env::var("ENCODED_PATH"))
        .unwrap_or("encoded".into())
});
