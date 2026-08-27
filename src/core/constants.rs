use std::{env, sync::LazyLock};

pub const DEFAULT_HOSTED_AUTH_PAGE: &str = "https://p2kr.github.io/komorebi-web/auth.html";
pub const VAULT_LOC: LazyLock<String> =
    LazyLock::new(|| env::var("VAULT_LOC").unwrap_or("vault".into()));
