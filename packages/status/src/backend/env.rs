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
