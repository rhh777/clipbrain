use regex::Regex;
use std::sync::LazyLock;

pub static URL_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^https?://\S+$").unwrap());

pub static EMAIL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[\w.+-]+@[\w-]+\.[\w.-]+$").unwrap());

pub static PHONE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^1[3-9]\d{9}$").unwrap());

pub static ID_CARD_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\d{17}[\dXx]$").unwrap());

pub static MATH_EXPR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[\d\s\+\-\*/\(\)\.\^%]+$").unwrap());
