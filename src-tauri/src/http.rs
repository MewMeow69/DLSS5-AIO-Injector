use std::os::windows::process::CommandExt;
use std::process::Command;
use std::sync::{Mutex, OnceLock};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

static TOKEN: OnceLock<Mutex<Option<String>>> = OnceLock::new();

fn token_slot() -> &'static Mutex<Option<String>> {
    TOKEN.get_or_init(|| Mutex::new(None))
}

pub fn set_github_token(token: Option<String>) {
    if let Ok(mut t) = token_slot().lock() {
        *t = token.filter(|s| !s.trim().is_empty());
    }
}

fn github_token() -> Option<String> {
    token_slot().lock().ok().and_then(|t| t.clone())
}

pub fn curl_get(url: &str) -> Option<String> {
    let mut args: Vec<String> = vec![
        "-s".into(),
        "-L".into(),
        "--max-time".into(),
        "25".into(),
        "-A".into(),
        "DLSS5-AIO-Injector/0.1".into(),
    ];
    if url.contains("api.github.com") {
        if let Some(t) = github_token() {
            args.push("-H".into());
            args.push(format!("Authorization: Bearer {}", t));
        }
    }
    args.push(url.to_string());
    let out = Command::new("curl.exe")
        .args(&args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    if !out.status.success() || out.stdout.is_empty() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).to_string())
}

pub fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}
