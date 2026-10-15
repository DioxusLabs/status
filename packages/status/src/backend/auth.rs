use anyhow::{anyhow, bail};
use hmac::{Hmac, Mac};
use sha2::Sha256;

use super::env;

type HmacSha256 = Hmac<Sha256>;

/// Admin session cookie lifetime (7 days).
const SESSION_MAX_AGE_SECS: u64 = 604800;

fn expected_cookie() -> Option<String> {
    let token = env::admin_token()?;
    let mut mac = HmacSha256::new_from_slice(token.as_bytes()).ok()?;
    mac.update(b"admin");
    Some(hex::encode(mac.finalize().into_bytes()))
}

pub fn is_admin() -> bool {
    let Some(expected) = expected_cookie() else {
        return false;
    };
    let Some(ctx) = dioxus::fullstack::FullstackContext::current() else {
        return false;
    };
    let headers = &ctx.parts_mut().headers;
    headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .map(|cookies| {
            cookies.split(';').any(|c| {
                let c = c.trim();
                c.strip_prefix("admin=") == Some(expected.as_str())
            })
        })
        .unwrap_or(false)
}

pub fn require_admin() -> anyhow::Result<()> {
    if is_admin() {
        Ok(())
    } else {
        Err(anyhow!("admin login required"))
    }
}

pub fn login(token: &str) -> anyhow::Result<()> {
    let Some(expected) = expected_cookie() else {
        bail!("ADMIN_TOKEN not configured on the server");
    };
    let mut mac = HmacSha256::new_from_slice(token.as_bytes())?;
    mac.update(b"admin");
    if hex::encode(mac.finalize().into_bytes()) != expected {
        bail!("invalid admin token");
    }
    if let Some(ctx) = dioxus::fullstack::FullstackContext::current() {
        ctx.add_response_header(
            dioxus::fullstack::http::header::SET_COOKIE,
            dioxus::fullstack::http::HeaderValue::from_str(&format!(
                "admin={expected}; Path=/; HttpOnly; SameSite=Lax; Max-Age={SESSION_MAX_AGE_SECS}"
            ))
            .unwrap(),
        );
    }
    Ok(())
}

pub fn logout() {
    if let Some(ctx) = dioxus::fullstack::FullstackContext::current() {
        ctx.add_response_header(
            dioxus::fullstack::http::header::SET_COOKIE,
            dioxus::fullstack::http::HeaderValue::from_static(
                "admin=; Path=/; HttpOnly; Max-Age=0",
            ),
        );
    }
}
