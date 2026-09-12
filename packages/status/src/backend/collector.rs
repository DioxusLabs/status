use std::collections::HashSet;
use std::sync::{Arc, OnceLock};

use tracing::{info, warn};

use super::{actions, crates_io, db, devin, env, github};

static GUARDS: OnceLock<
    std::sync::Mutex<std::collections::HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
> = OnceLock::new();

fn guard_for(kind: &str) -> Arc<tokio::sync::Mutex<()>> {
    GUARDS
        .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
        .lock()
        .unwrap()
        .entry(kind.to_string())
        .or_default()
        .clone()
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

pub async fn run_sync(kind: &str) -> anyhow::Result<String> {
    let guard = guard_for(kind);
    let _lock = guard
        .try_lock()
        .map_err(|_| anyhow::anyhow!("a '{kind}' sync is already running"))?;
    let started = now();
    let result = run_inner(kind).await;
    match &result {
        Ok(msg) => db::log_sync(kind, "", &started, true, msg).await?,
        Err(e) => db::log_sync(kind, "", &started, false, &format!("{e:#}")).await?,
    }
    result
}

async fn run_inner(kind: &str) -> anyhow::Result<String> {
    match kind {
        "repos" => sync_repos().await,
        "prs" | "issues" => sync_prs_issues().await,
        "crates" => {
            let n = crates_io::sync_crates().await?;
            Ok(format!("{n} crates synced"))
        }
        "releases" => sync_releases().await,
        "snapshots" => {
            take_snapshots().await?;
            Ok("snapshots written".into())
        }
        "all" => {
            let r = sync_repos().await;
            let p = sync_prs_issues().await;
            let c = crates_io::sync_crates().await;
            let rel = sync_releases().await;
            let mut msgs = vec![];
            for (name, res) in [
                ("repos", r.map(|_| ())),
                ("prs", p.map(|_| ())),
                ("crates", c.map(|_| ())),
                ("releases", rel.map(|_| ())),
            ] {
                match res {
                    Ok(()) => msgs.push(format!("{name} ok")),
                    Err(e) => msgs.push(format!("{name}: {e:#}")),
                }
            }
            Ok(msgs.join("; "))
        }
        other => Err(anyhow::anyhow!("unknown sync kind '{other}'")),
    }
}

async fn sync_repos() -> anyhow::Result<String> {
    let repos = github::list_org_repos().await?;
    let n = repos.len();
    for r in &repos {
        db::upsert_repo(r).await?;
    }
    db::seed_monitored().await?;
    Ok(format!("{n} repos synced"))
}

async fn sync_prs_issues() -> anyhow::Result<String> {
    let members = github::org_members().await.unwrap_or_else(|e| {
        warn!("failed to fetch org members: {e:#}");
        HashSet::new()
    });
    let repos = db::monitored_repos().await?;
    let mut total = 0;
    for repo in &repos {
        match github::sync_open_prs(repo, &members).await {
            Ok(n) => {
                total += n;
                db::log_sync("prs", repo, &now(), true, &format!("{n} open prs")).await?;
            }
            Err(e) => {
                warn!("sync prs {repo}: {e:#}");
                db::log_sync("prs", repo, &now(), false, &format!("{e:#}")).await?;
            }
        }
        match github::sync_closed_prs(repo).await {
            Ok(n) if n > 0 => {
                db::log_sync("prs", repo, &now(), true, &format!("{n} closed/merged prs")).await?
            }
            Ok(_) => {}
            Err(e) => warn!("sync closed prs {repo}: {e:#}"),
        }
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        match github::sync_open_issues(repo, &members).await {
            Ok(n) => {
                db::log_sync("issues", repo, &now(), true, &format!("{n} open issues")).await?
            }
            Err(e) => {
                warn!("sync issues {repo}: {e:#}");
                db::log_sync("issues", repo, &now(), false, &format!("{e:#}")).await?;
            }
        }
        match github::sync_closed_issues(repo).await {
            Ok(n) if n > 0 => {
                db::log_sync("issues", repo, &now(), true, &format!("{n} closed issues")).await?
            }
            Ok(_) => {}
            Err(e) => warn!("sync closed issues {repo}: {e:#}"),
        }
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    }
    Ok(format!("{total} open prs across {} repos", repos.len()))
}

async fn sync_releases() -> anyhow::Result<String> {
    let repos = db::monitored_repos().await?;
    let mut total = 0;
    for repo in &repos {
        match github::sync_releases(repo).await {
            Ok(n) => total += n,
            Err(e) => warn!("sync releases {repo}: {e:#}"),
        }
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }
    Ok(format!("{total} releases"))
}

async fn take_snapshots() -> anyhow::Result<()> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    for r in db::list_repos().await? {
        sqlx::query(
            "INSERT INTO repo_snapshots (repo, date, stars, forks, open_issues, open_prs)
             VALUES (?,?,?,?,?,?)
             ON CONFLICT(repo, date) DO UPDATE SET stars=excluded.stars, forks=excluded.forks,
                open_issues=excluded.open_issues, open_prs=excluded.open_prs",
        )
        .bind(&r.name)
        .bind(&today)
        .bind(r.stars)
        .bind(r.forks)
        .bind(r.open_issues)
        .bind(r.open_prs)
        .execute(db::pool())
        .await?;
    }
    Ok(())
}

fn spawn_periodic(kind: &'static str, interval: std::time::Duration, run_immediately: bool) {
    tokio::spawn(async move {
        if run_immediately {
            if let Err(e) = run_sync(kind).await {
                warn!("initial {kind} sync failed: {e:#}");
            }
        }
        loop {
            tokio::time::sleep(interval).await;
            if let Err(e) = run_sync(kind).await {
                warn!("{kind} sync failed: {e:#}");
            }
        }
    });
}

pub fn start() {
    if env::github_token().is_none() {
        warn!("GITHUB_TOKEN not set; collectors disabled");
        return;
    }
    tokio::spawn(async move {
        if db::repos_empty().await.unwrap_or(true) {
            info!("repos table empty; running initial sync");
            let _ = run_sync("all").await;
        }
    });
    spawn_periodic("repos", env::repos_interval(), false);
    spawn_periodic("prs", env::prs_interval(), true);
    spawn_periodic("crates", env::crates_interval(), true);
    spawn_periodic("releases", env::releases_interval(), false);
    spawn_periodic("snapshots", env::snapshots_interval(), true);
    spawn_devin_poller();
    info!("collectors started");
}

/// Poll in-flight Devin sessions every 60s; the only background work that
/// touches the Devin API.
fn spawn_devin_poller() {
    if !devin::configured() {
        return;
    }
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            match db::pending_devin_sessions(20).await {
                Ok(rows) => {
                    for row in rows {
                        if let Err(e) = actions::poll_session(&row).await {
                            warn!("devin poll {}: {e:#}", row.session_id);
                            let _ = db::update_devin_session(
                                &row.session_id,
                                "error",
                                None,
                                None,
                                None,
                                Some(&format!("{e:#}")),
                            )
                            .await;
                        }
                    }
                }
                Err(e) => warn!("devin poll list: {e:#}"),
            }
        }
    });
}
