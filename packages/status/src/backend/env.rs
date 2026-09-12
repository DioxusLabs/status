use std::path::PathBuf;

pub fn github_token() -> Option<String> {
    std::env::var("GITHUB_TOKEN").ok().filter(|t| !t.is_empty())
}

pub fn admin_token() -> Option<String> {
    std::env::var("ADMIN_TOKEN").ok().filter(|t| !t.is_empty())
}

pub fn data_dir() -> PathBuf {
    std::env::var("DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./_data"))
}

pub fn port() -> Option<u16> {
    std::env::var("PORT").ok().and_then(|p| p.parse().ok())
}

pub fn org() -> String {
    std::env::var("GITHUB_ORG").unwrap_or_else(|_| "DioxusLabs".into())
}

pub fn github_app_id() -> Option<u64> {
    std::env::var("GITHUB_APP_ID")
        .ok()
        .and_then(|v| v.parse().ok())
}

/// PEM contents from GITHUB_APP_PRIVATE_KEY or the file at
/// GITHUB_APP_PRIVATE_KEY_PATH; literal `\n` escapes are normalized.
pub fn github_app_private_key() -> Option<Vec<u8>> {
    if let Ok(pem) = std::env::var("GITHUB_APP_PRIVATE_KEY") {
        if !pem.is_empty() {
            return Some(normalize_pem(&pem).into_bytes());
        }
    }
    let path = std::env::var("GITHUB_APP_PRIVATE_KEY_PATH").ok()?;
    if path.is_empty() {
        return None;
    }
    std::fs::read(&path)
        .ok()
        .map(|b| normalize_pem(String::from_utf8_lossy(&b).as_ref()).into_bytes())
}

pub fn normalize_pem(pem: &str) -> String {
    pem.replace("\\n", "\n")
}

pub fn github_app_installation_id() -> Option<u64> {
    std::env::var("GITHUB_APP_INSTALLATION_ID")
        .ok()
        .and_then(|v| v.parse().ok())
}

pub fn devin_api_key() -> Option<String> {
    std::env::var("DEVIN_API_KEY")
        .ok()
        .filter(|k| !k.is_empty())
}

pub fn devin_api_base() -> String {
    let b = std::env::var("DEVIN_API_BASE").unwrap_or_else(|_| "https://api.devin.ai".into());
    b.trim_end_matches('/').to_string()
}

pub fn devin_org_id() -> Option<String> {
    std::env::var("DEVIN_ORG_ID").ok().filter(|v| !v.is_empty())
}

pub fn llm_daily_budget() -> i64 {
    std::env::var("LLM_DAILY_BUDGET")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(50)
}

fn interval_secs(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

pub fn repos_interval() -> std::time::Duration {
    std::time::Duration::from_secs(interval_secs("SYNC_REPOS_SECS", 6 * 3600))
}

pub fn prs_interval() -> std::time::Duration {
    std::time::Duration::from_secs(interval_secs("SYNC_PRS_SECS", 10 * 60))
}

pub fn crates_interval() -> std::time::Duration {
    std::time::Duration::from_secs(interval_secs("SYNC_CRATES_SECS", 3600))
}

pub fn releases_interval() -> std::time::Duration {
    std::time::Duration::from_secs(interval_secs("SYNC_RELEASES_SECS", 6 * 3600))
}

pub fn snapshots_interval() -> std::time::Duration {
    std::time::Duration::from_secs(interval_secs("SYNC_SNAPSHOTS_SECS", 6 * 3600))
}

#[cfg(test)]
mod tests {
    #[test]
    fn pem_literal_newlines_normalized() {
        let raw = "-----BEGIN A-----\\nabc\\n-----END A-----";
        let out = super::normalize_pem(raw);
        assert_eq!(out.lines().count(), 3);
        assert!(!out.contains("\\n"));
    }
}
