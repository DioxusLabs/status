use std::collections::HashMap;
use std::time::Duration;

use anyhow::Context;
use serde::Deserialize;

use super::db;

const UA: &str = "dioxus-status (github.com/DioxusLabs/status)";

pub const DEFAULT_CRATES: &[&str] = &[
    "dioxus",
    "dioxus-core",
    "dioxus-cli",
    "dioxus-web",
    "dioxus-desktop",
    "dioxus-fullstack",
    "dioxus-router",
    "dioxus-signals",
    "dioxus-html",
    "dioxus-hooks",
    "dioxus-liveview",
    "dioxus-native",
    "dioxus-ssr",
    "taffy",
    "blitz-dom",
    "blitz-html",
    "stylo",
    "dioxus-primitives",
    "manganis",
    "dioxus-sdk",
    "subsecond",
];

pub fn crate_to_repo(name: &str) -> &'static str {
    match name {
        "dioxus-primitives" | "dioxus-attributes" => "dioxus-components",
        "manganis" | "subsecond" => "dioxus",
        "dioxus-sdk" | "dioxus-time" => "sdk",
        n if n.starts_with("dioxus") => "dioxus",
        n if n.starts_with("blitz") => "blitz",
        "taffy" => "taffy",
        "stylo" | "parley" | "vello" | "peniko" | "anyrender" | "color" => "anyrender",
        _ => "dioxus",
    }
}

fn client() -> anyhow::Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(UA)
        .timeout(Duration::from_secs(30))
        .build()
        .context("build reqwest client")
}

#[derive(Deserialize)]
struct CrateResp {
    #[serde(rename = "crate")]
    krate: CrateInfo,
}

#[derive(Deserialize)]
struct CrateInfo {
    downloads: Option<i64>,
    recent_downloads: Option<i64>,
    max_version: Option<String>,
    newest_version: Option<String>,
}

#[derive(Deserialize)]
struct VersionsResp {
    versions: Vec<VersionInfo>,
}

#[derive(Deserialize)]
struct VersionInfo {
    num: String,
    downloads: i64,
    created_at: String,
}

#[derive(Deserialize)]
struct DownloadsResp {
    version_downloads: Vec<DayDownload>,
    meta: Option<DownloadsMeta>,
}

#[derive(Deserialize)]
struct DayDownload {
    date: String,
    downloads: i64,
}

#[derive(Deserialize)]
struct DownloadsMeta {
    extra_downloads: Option<Vec<DayDownload>>,
}

pub async fn crate_list() -> anyhow::Result<Vec<String>> {
    match db::get_setting("crates").await? {
        Some(v) => Ok(serde_json::from_str(&v)
            .unwrap_or_else(|_| DEFAULT_CRATES.iter().map(|s| s.to_string()).collect())),
        None => {
            let list: Vec<String> = DEFAULT_CRATES.iter().map(|s| s.to_string()).collect();
            db::set_setting("crates", &serde_json::to_string(&list)?).await?;
            Ok(list)
        }
    }
}

pub async fn sync_crates() -> anyhow::Result<usize> {
    let http = client()?;
    let names = crate_list().await?;
    let mut count = 0;
    for name in names {
        if let Err(e) = sync_one(&http, &name).await {
            tracing::warn!("crates.io sync {name}: {e:#}");
        } else {
            count += 1;
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    Ok(count)
}

async fn sync_one(http: &reqwest::Client, name: &str) -> anyhow::Result<()> {
    let info: CrateResp = http
        .get(format!("https://crates.io/api/v1/crates/{name}"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    sqlx::query(
        "INSERT INTO crates (name, repo, total_downloads, recent_downloads, latest_version, synced_at)
         VALUES (?,?,?,?,?,?)
         ON CONFLICT(name) DO UPDATE SET repo=excluded.repo, total_downloads=excluded.total_downloads,
            recent_downloads=excluded.recent_downloads, latest_version=excluded.latest_version,
            synced_at=excluded.synced_at",
    )
    .bind(name)
    .bind(crate_to_repo(name))
    .bind(info.krate.downloads.unwrap_or(0))
    .bind(info.krate.recent_downloads.unwrap_or(0))
    .bind(
        info.krate
            .max_version
            .or(info.krate.newest_version)
            .unwrap_or_default(),
    )
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(db::pool())
    .await?;

    let versions: VersionsResp = http
        .get(format!(
            "https://crates.io/api/v1/crates/{name}/versions?per_page=50"
        ))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let mut latest_at: Option<String> = None;
    for v in &versions.versions {
        sqlx::query(
            "INSERT INTO crate_versions (crate_name, version, downloads, created_at) VALUES (?,?,?,?)
             ON CONFLICT(crate_name, version) DO UPDATE SET downloads=excluded.downloads, created_at=excluded.created_at",
        )
        .bind(name)
        .bind(&v.num)
        .bind(v.downloads)
        .bind(&v.created_at)
        .execute(db::pool())
        .await?;
        if latest_at
            .as_deref()
            .is_none_or(|cur| v.created_at.as_str() > cur)
        {
            latest_at = Some(v.created_at.clone());
        }
    }
    if let Some(at) = latest_at {
        sqlx::query("UPDATE crates SET latest_version_at = ? WHERE name = ?")
            .bind(at)
            .bind(name)
            .execute(db::pool())
            .await?;
    }

    let daily: DownloadsResp = http
        .get(format!("https://crates.io/api/v1/crates/{name}/downloads"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let mut per_day: HashMap<String, i64> = HashMap::new();
    for d in daily.version_downloads {
        *per_day.entry(d.date).or_default() += d.downloads;
    }
    if let Some(extra) = daily.meta.and_then(|m| m.extra_downloads) {
        for d in extra {
            *per_day.entry(d.date).or_default() += d.downloads;
        }
    }
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(90))
        .format("%Y-%m-%d")
        .to_string();
    for (date, downloads) in per_day {
        if date >= cutoff {
            sqlx::query(
                "INSERT INTO crate_downloads_daily (crate_name, date, downloads) VALUES (?,?,?)
                 ON CONFLICT(crate_name, date) DO UPDATE SET downloads=excluded.downloads",
            )
            .bind(name)
            .bind(&date)
            .bind(downloads)
            .execute(db::pool())
            .await?;
        }
    }
    Ok(())
}
