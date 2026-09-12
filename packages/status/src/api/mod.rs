#[cfg(feature = "server")]
use anyhow::Context;
use dioxus::prelude::*;

use crate::model::*;

#[cfg(feature = "server")]
use crate::backend::{actions, auth, collector, db, devin, env};

#[cfg(feature = "server")]
fn pr_summary(p: &db::DbPr) -> PrSummary {
    let row = p.into_row();
    let age_days = row
        .created_at
        .as_deref()
        .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
        .map(|t| (chrono::Utc::now() - t.with_timezone(&chrono::Utc)).num_days())
        .unwrap_or(0);
    PrSummary {
        repo: row.repo,
        number: row.number,
        title: row.title,
        url: row.url,
        author: row.author,
        age_days,
        score: row.score,
        ci_state: row.ci_state,
    }
}

#[post("/api/overview")]
pub async fn get_overview(repos: Option<Vec<String>>) -> Result<Overview> {
    let mut o = Overview::default();
    let (name_clause, _) = repo_clause("name", repos.as_deref());
    let (repo_clause, repo_binds) = repo_clause("repo", repos.as_deref());

    o.monitored_repos = scalar("SELECT COUNT(*) FROM repos WHERE monitored = 1", &[]).await?;
    o.open_prs = scalar(
        &format!("SELECT COUNT(*) FROM pull_requests WHERE state = 'open'{repo_clause}"),
        &repo_binds,
    )
    .await?;
    o.open_issues = scalar(
        &format!("SELECT COUNT(*) FROM issues WHERE state = 'open'{repo_clause}"),
        &repo_binds,
    )
    .await?;
    o.total_stars = scalar(
        &format!("SELECT COALESCE(SUM(stars),0) FROM repos WHERE monitored = 1{name_clause}"),
        &repo_binds,
    )
    .await?;
    o.downloads_7d = scalar(
        &format!(
            "SELECT COALESCE(SUM(d.downloads),0) FROM crate_downloads_daily d
         JOIN crates c ON c.name = d.crate_name
         WHERE d.date >= date('now', '-7 days'){repo_clause}"
        ),
        &repo_binds,
    )
    .await?;
    let has_week_old_snapshot: bool = scalar(
        &format!("SELECT EXISTS(SELECT 1 FROM repo_snapshots WHERE date <= date('now','-7 days'){repo_clause})"),
        &repo_binds,
    )
    .await?;
    if has_week_old_snapshot {
        let delta_binds: Vec<String> = (0..4).flat_map(|_| repo_binds.iter().cloned()).collect();
        o.stars_7d_delta = scalar(
            &format!("SELECT (SELECT SUM(stars) FROM repo_snapshots WHERE date = (SELECT MAX(date) FROM repo_snapshots WHERE 1=1{repo_clause}){repo_clause})
                  - (SELECT SUM(stars) FROM repo_snapshots WHERE date = (SELECT MAX(date) FROM repo_snapshots WHERE date <= date('now','-7 days'){repo_clause}){repo_clause})"),
            &delta_binds,
        )
        .await?;
    }

    o.ready_to_merge = top_prs(
        format!("SELECT * FROM pull_requests WHERE state='open' AND is_draft=0 AND ci_state='success'{repo_clause}
         AND review_decision='APPROVED' AND mergeable='MERGEABLE' AND score >= 60
         ORDER BY score DESC LIMIT 10"), repo_binds.clone(),
    ).await?;
    o.quick_wins = top_prs(
        format!("SELECT * FROM pull_requests WHERE state='open' AND is_draft=0 AND ci_state='success'{repo_clause}
         AND (additions + deletions) <= 50 ORDER BY score DESC LIMIT 10"), repo_binds.clone(),
    ).await?;
    o.going_stale = top_prs(
        format!(
            "SELECT * FROM pull_requests WHERE state='open' AND is_draft=0{repo_clause}
         AND last_activity_by = 'contributor' AND author_association NOT IN ('MEMBER','OWNER')
         AND last_activity_at < datetime('now', '-14 days') ORDER BY last_activity_at ASC LIMIT 10"
        ),
        repo_binds.clone(),
    )
    .await?;
    o.waiting_on_maintainer = top_prs(
        format!("SELECT * FROM pull_requests WHERE state='open' AND last_activity_by = 'contributor'{repo_clause}
         ORDER BY last_activity_at ASC LIMIT 10"), repo_binds.clone(),
    ).await?;
    o.new_this_week = top_prs(
        format!("SELECT * FROM pull_requests WHERE state='open' AND created_at > datetime('now', '-7 days'){repo_clause}
         ORDER BY created_at DESC LIMIT 10"), repo_binds.clone(),
    ).await?;
    o.first_time_contributors = top_prs(
        format!(
            "SELECT * FROM pull_requests WHERE state='open'{repo_clause}
         AND author_association IN ('FIRST_TIME_CONTRIBUTOR','FIRST_TIMER')
         ORDER BY created_at DESC LIMIT 10"
        ),
        repo_binds,
    )
    .await?;
    o.last_sync = db::last_sync().await?;
    Ok(o)
}

#[cfg(feature = "server")]
fn repo_clause(column: &str, repos: Option<&[String]>) -> (String, Vec<String>) {
    let Some(repos) = repos else {
        return (String::new(), Vec::new());
    };
    if repos.is_empty() {
        return (" AND 1=0".into(), Vec::new());
    }
    let placeholders = std::iter::repeat_n("?", repos.len())
        .collect::<Vec<_>>()
        .join(",");
    (format!(" AND {column} IN ({placeholders})"), repos.to_vec())
}

#[cfg(feature = "server")]
async fn scalar<T>(sql: &str, binds: &[String]) -> anyhow::Result<T>
where
    T: for<'r> sqlx::Decode<'r, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite> + Send + Unpin,
{
    let mut query = sqlx::query_scalar::<_, T>(sql);
    for bind in binds {
        query = query.bind(bind);
    }
    Ok(query.fetch_one(db::pool()).await?)
}

#[cfg(feature = "server")]
async fn top_prs(sql: String, binds: Vec<String>) -> anyhow::Result<Vec<PrSummary>> {
    let mut query = sqlx::query_as::<_, db::DbPr>(&sql);
    for bind in binds {
        query = query.bind(bind);
    }
    let rows = query.fetch_all(db::pool()).await?;
    Ok(rows.iter().map(pr_summary).take(10).collect())
}

#[cfg(feature = "server")]
fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

#[post("/api/prs")]
pub async fn list_prs(filter: PrFilter) -> Result<Vec<PrRow>> {
    use crate::backend::query::parse_query;
    let parsed = parse_query(&filter.query);

    let mut sql = String::from("SELECT * FROM pull_requests WHERE 1=1");
    let mut binds: Vec<String> = vec![];

    let state = filter.state.clone().unwrap_or_else(|| "open".into());
    sql.push_str(" AND state = ?");
    binds.push(state);

    let mut repos = filter.repos.clone();
    repos.extend(parsed.repos.clone());
    if !repos.is_empty() {
        sql.push_str(&format!(
            " AND repo IN ({})",
            repos.iter().map(|_| "?").collect::<Vec<_>>().join(",")
        ));
        binds.extend(repos);
    }
    for a in &parsed.authors {
        sql.push_str(" AND author = ?");
        binds.push(a.clone());
    }
    for l in &parsed.labels {
        sql.push_str(" AND labels_json LIKE ? ESCAPE '\\'");
        binds.push(format!("%\"{}%", escape_like(l)));
    }
    for l in &parsed.exclude_labels {
        sql.push_str(" AND labels_json NOT LIKE ? ESCAPE '\\'");
        binds.push(format!("%\"{}%", escape_like(l)));
    }
    for f in &parsed.is_flags {
        match f.as_str() {
            "draft" => sql.push_str(" AND is_draft = 1"),
            "green" => sql.push_str(" AND ci_state = 'success'"),
            "red" => sql.push_str(" AND ci_state = 'failure'"),
            "approved" => sql.push_str(" AND review_decision = 'APPROVED'"),
            "conflict" => sql.push_str(" AND mergeable = 'CONFLICTING'"),
            "stale" => sql.push_str(" AND last_activity_at < datetime('now','-14 days')"),
            "first-timer" => {
                sql.push_str(" AND author_association IN ('FIRST_TIME_CONTRIBUTOR','FIRST_TIMER')")
            }
            _ => {}
        }
    }
    if !parsed.text.is_empty() {
        sql.push_str(" AND (title LIKE ? ESCAPE '\\' OR body LIKE ? ESCAPE '\\')");
        let pat = format!("%{}%", escape_like(&parsed.text));
        binds.push(pat.clone());
        binds.push(pat);
    }
    if let Some(d) = filter.draft {
        sql.push_str(if d {
            " AND is_draft = 1"
        } else {
            " AND is_draft = 0"
        });
    }
    if let Some(ci) = &filter.ci {
        sql.push_str(" AND ci_state = ?");
        binds.push(ci.clone());
    }
    if let Some(r) = &filter.review {
        sql.push_str(" AND review_decision = ?");
        binds.push(r.clone());
    }
    if let Some(m) = filter.mergeable {
        sql.push_str(if m {
            " AND mergeable = 'MERGEABLE'"
        } else {
            " AND mergeable = 'CONFLICTING'"
        });
    }
    if let Some(a) = &filter.association {
        sql.push_str(" AND author_association = ?");
        binds.push(a.clone());
    }
    if let Some(s) = &filter.size {
        let cond = match s.as_str() {
            "xs" => "(additions + deletions) <= 20",
            "s" => "(additions + deletions) BETWEEN 21 AND 100",
            "m" => "(additions + deletions) BETWEEN 101 AND 400",
            "l" => "(additions + deletions) BETWEEN 401 AND 1000",
            _ => "(additions + deletions) > 1000",
        };
        sql.push_str(&format!(" AND {cond}"));
    }
    if let Some(days) = filter.min_age_days {
        sql.push_str(&format!(" AND created_at < datetime('now','-{days} days')"));
    }
    if let Some(days) = filter.stale_days {
        sql.push_str(&format!(
            " AND last_activity_at < datetime('now','-{days} days')"
        ));
    }
    if filter.first_timers_only {
        sql.push_str(" AND author_association IN ('FIRST_TIME_CONTRIBUTOR','FIRST_TIMER')");
    }
    sql.push_str(match filter.sort {
        PrSort::Score => " ORDER BY score DESC",
        PrSort::Attention => " ORDER BY (last_activity_by = 'contributor') DESC, score ASC",
        PrSort::Newest => " ORDER BY created_at DESC",
        PrSort::Oldest => " ORDER BY created_at ASC",
        PrSort::LastActivity => " ORDER BY last_activity_at DESC",
        PrSort::Size => " ORDER BY (additions + deletions) DESC",
    });
    let limit = filter.limit.clamp(1, 500);
    sql.push_str(&format!(" LIMIT {limit} OFFSET {}", filter.offset.max(0)));

    let mut q = sqlx::query_as::<_, db::DbPr>(&sql);
    for b in binds {
        q = q.bind(b);
    }
    let rows = q.fetch_all(db::pool()).await?;
    Ok(rows.iter().map(|r| r.into_row()).collect())
}

#[get("/api/pr/:repo/:number")]
pub async fn get_pr(repo: String, number: i64) -> Result<PrDetail> {
    Ok(db::get_pr(&repo, number)
        .await?
        .context(format!("pr {repo}#{number} not found"))?)
}

#[post("/api/issues")]
pub async fn list_issues(filter: IssueFilter) -> Result<Vec<IssueRow>> {
    use crate::backend::query::parse_query;
    let parsed = parse_query(&filter.query);
    let mut sql = String::from("SELECT * FROM issues WHERE 1=1");
    let mut binds: Vec<String> = vec![];
    let state = filter.state.clone().unwrap_or_else(|| "open".into());
    sql.push_str(" AND state = ?");
    binds.push(state);
    let mut repos = filter.repos.clone();
    repos.extend(parsed.repos.clone());
    if !repos.is_empty() {
        sql.push_str(&format!(
            " AND repo IN ({})",
            repos.iter().map(|_| "?").collect::<Vec<_>>().join(",")
        ));
        binds.extend(repos);
    }
    for a in &parsed.authors {
        sql.push_str(" AND author = ?");
        binds.push(a.clone());
    }
    if !parsed.text.is_empty() {
        sql.push_str(" AND (title LIKE ? ESCAPE '\\' OR body LIKE ? ESCAPE '\\')");
        let pat = format!("%{}%", escape_like(&parsed.text));
        binds.push(pat.clone());
        binds.push(pat);
    }
    sql.push_str(match filter.sort.as_deref() {
        Some("oldest") => " ORDER BY created_at ASC",
        Some("comments") => " ORDER BY comments DESC",
        Some("reactions") => " ORDER BY reactions DESC",
        _ => " ORDER BY updated_at DESC",
    });
    let limit = filter.limit.clamp(1, 500);
    sql.push_str(&format!(" LIMIT {limit} OFFSET {}", filter.offset.max(0)));
    let mut q = sqlx::query_as::<_, db::DbIssue>(&sql);
    for b in binds {
        q = q.bind(b);
    }
    let rows = q.fetch_all(db::pool()).await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

#[get("/api/repos")]
pub async fn list_repos() -> Result<Vec<RepoRow>> {
    db::list_repos().await.map_err(Into::into)
}

#[get("/api/repo/:name")]
pub async fn get_repo(name: String) -> Result<RepoDetail> {
    let repo = db::get_repo(&name)
        .await?
        .context(format!("repo {name} not found"))?;
    let pool = db::pool();
    let open_prs =
        sqlx::query_scalar("SELECT COUNT(*) FROM pull_requests WHERE repo=? AND state='open'")
            .bind(&name)
            .fetch_one(pool)
            .await?;
    let open_issues =
        sqlx::query_scalar("SELECT COUNT(*) FROM issues WHERE repo=? AND state='open'")
            .bind(&name)
            .fetch_one(pool)
            .await?;
    let crates = sqlx::query_as::<_, (String, String, i64, i64, Option<String>, Option<String>)>(
        "SELECT name, repo, total_downloads, recent_downloads, latest_version, latest_version_at
         FROM crates WHERE repo = ? ORDER BY total_downloads DESC",
    )
    .bind(&name)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| CrateRow {
        name: r.0,
        repo: r.1,
        total_downloads: r.2,
        recent_downloads: r.3,
        latest_version: r.4.unwrap_or_default(),
        latest_version_at: r.5,
    })
    .collect();
    let releases = sqlx::query_as::<
        _,
        (
            String,
            String,
            Option<String>,
            Option<String>,
            String,
            bool,
            String,
        ),
    >(
        "SELECT repo, tag, name, published_at, url, is_prerelease, assets_json
         FROM releases WHERE repo = ? ORDER BY published_at DESC LIMIT 20",
    )
    .bind(&name)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| ReleaseRow {
        repo: r.0,
        tag: r.1,
        name: r.2.unwrap_or_default(),
        published_at: r.3,
        url: r.4,
        is_prerelease: r.5,
        assets: serde_json::from_str(&r.6).unwrap_or_default(),
    })
    .collect();
    let stars_trend = sqlx::query_as::<_, (String, i64)>(
        "SELECT date, stars FROM repo_snapshots WHERE repo = ? ORDER BY date",
    )
    .bind(&name)
    .fetch_all(pool)
    .await?;
    let recent_prs = sqlx::query_as::<_, db::DbPr>(
        "SELECT * FROM pull_requests WHERE repo = ? AND state='open' ORDER BY updated_at DESC LIMIT 15",
    )
    .bind(&name)
    .fetch_all(pool)
    .await?
    .iter()
    .map(|r| r.into_row())
    .collect();
    let recent_issues = sqlx::query_as::<_, db::DbIssue>(
        "SELECT * FROM issues WHERE repo = ? AND state='open' ORDER BY updated_at DESC LIMIT 15",
    )
    .bind(&name)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(Into::into)
    .collect();
    Ok(RepoDetail {
        repo,
        open_prs,
        open_issues,
        crates,
        releases,
        stars_trend,
        recent_prs,
        recent_issues,
    })
}

#[get("/api/health")]
pub async fn get_health() -> Result<Health> {
    let pool = db::pool();
    let crate_names: Vec<(String, i64, i64, Option<String>)> = sqlx::query_as(
        "SELECT name, total_downloads, recent_downloads, latest_version FROM crates ORDER BY total_downloads DESC",
    )
    .fetch_all(pool)
    .await?;
    let mut crates = Vec::new();
    for (name, total, recent, latest) in crate_names {
        let daily = sqlx::query_as::<_, (String, i64)>(
            "SELECT date, downloads FROM crate_downloads_daily WHERE crate_name = ? ORDER BY date",
        )
        .bind(&name)
        .fetch_all(pool)
        .await?;
        crates.push(CrateSeries {
            name,
            daily,
            total,
            recent,
            latest_version: latest.unwrap_or_default(),
        });
    }
    let repos: Vec<(String,)> = sqlx::query_as("SELECT name FROM repos WHERE monitored = 1")
        .fetch_all(pool)
        .await?;
    let mut stars_trend = Vec::new();
    for (repo,) in repos {
        let series = sqlx::query_as::<_, (String, i64)>(
            "SELECT date, stars FROM repo_snapshots WHERE repo = ? ORDER BY date",
        )
        .bind(&repo)
        .fetch_all(pool)
        .await?;
        if !series.is_empty() {
            stars_trend.push((repo, series));
        }
    }
    Ok(Health {
        crates,
        stars_trend,
        open_prs: sqlx::query_scalar("SELECT COUNT(*) FROM pull_requests WHERE state='open'")
            .fetch_one(pool)
            .await?,
        open_issues: sqlx::query_scalar("SELECT COUNT(*) FROM issues WHERE state='open'")
            .fetch_one(pool)
            .await?,
        prs_merged_30d: sqlx::query_scalar(
            "SELECT COUNT(*) FROM pull_requests WHERE merged_at > datetime('now','-30 days')",
        )
        .fetch_one(pool)
        .await?,
        issues_closed_30d: sqlx::query_scalar(
            "SELECT COUNT(*) FROM issues WHERE closed_at > datetime('now','-30 days')",
        )
        .fetch_one(pool)
        .await?,
    })
}

#[get("/api/settings")]
pub async fn get_settings() -> Result<SettingsView> {
    let repos = db::list_repos().await?;
    let crates: Vec<String> = db::get_setting("crates")
        .await?
        .and_then(|v| serde_json::from_str(&v).ok())
        .unwrap_or_else(|| {
            crate::backend::crates_io::DEFAULT_CRATES
                .iter()
                .map(|s| s.to_string())
                .collect()
        });
    Ok(SettingsView {
        is_admin: auth::is_admin(),
        repos,
        crates,
        sync_log: db::sync_log(50).await?,
        devin_configured: devin::configured(),
        devin_budget_used: db::budget_used_today().await.unwrap_or(0),
        devin_budget_total: db::budget_limit().await.unwrap_or(0),
    })
}

#[post("/api/admin/login")]
pub async fn admin_login(token: String) -> Result<bool> {
    auth::login(&token)?;
    Ok(true)
}

#[post("/api/admin/logout")]
pub async fn admin_logout() -> Result<()> {
    auth::logout();
    Ok(())
}

#[get("/api/admin")]
pub async fn is_admin() -> Result<bool> {
    Ok(auth::is_admin())
}

#[post("/api/settings/monitored")]
pub async fn set_monitored_repos(names: Vec<String>) -> Result<()> {
    auth::require_admin()?;
    db::set_monitored(&names).await?;
    Ok(())
}

#[post("/api/settings/crates")]
pub async fn set_crates(names: Vec<String>) -> Result<()> {
    auth::require_admin()?;
    db::set_setting("crates", &serde_json::to_string(&names)?).await?;
    Ok(())
}

#[post("/api/sync/:kind")]
pub async fn sync_now(kind: String) -> Result<String> {
    auth::require_admin()?;
    collector::run_sync(&kind).await.map_err(Into::into)
}

#[post("/api/settings/budget")]
pub async fn set_llm_budget(value: i64) -> Result<()> {
    auth::require_admin()?;
    db::set_setting("llm_daily_budget", &value.to_string()).await?;
    Ok(())
}

#[post("/api/devin/dispatch")]
pub async fn dispatch_action(
    repo: String,
    number: i64,
    kind: String,
    custom_prompt: Option<String>,
) -> Result<DevinSessionRow> {
    auth::require_admin()?;
    let kind: ActionKind = kind.parse()?;
    actions::dispatch(&repo, number, kind, custom_prompt.as_deref())
        .await
        .map_err(Into::into)
}

#[post("/api/devin/assess")]
pub async fn assess_pr(repo: String, number: i64, force: bool) -> Result<AssessmentView> {
    auth::require_admin()?;
    actions::dispatch_assess(&repo, number, force)
        .await
        .map_err(Into::into)
}

#[post("/api/devin/assessment")]
pub async fn get_assessment(repo: String, number: i64) -> Result<Option<AssessmentView>> {
    let pr = db::get_pr(&repo, number).await?;
    match pr {
        Some(p) => db::get_assessment(&repo, number, &p.head_sha)
            .await
            .map_err(Into::into),
        None => Ok(None),
    }
}

#[post("/api/devin/sessions")]
pub async fn list_sessions(
    repo: Option<String>,
    number: Option<i64>,
    limit: i64,
) -> Result<Vec<DevinSessionRow>> {
    let mut rows = db::list_devin_sessions(repo.as_deref(), number, limit).await?;
    if !auth::is_admin() {
        for r in &mut rows {
            r.prompt.clear();
        }
    }
    Ok(rows)
}

#[post("/api/devin/refresh")]
pub async fn refresh_session(session_id: String) -> Result<DevinSessionRow> {
    auth::require_admin()?;
    let row = db::get_devin_session(&session_id)
        .await?
        .context("unknown session")?;
    actions::poll_session(&row).await?;
    db::get_devin_session(&session_id)
        .await?
        .context("unknown session")
        .map_err(Into::into)
}

#[post("/api/devin/message")]
pub async fn send_session_message(session_id: String, message: String) -> Result<()> {
    auth::require_admin()?;
    devin::send_message(&session_id, &message)
        .await
        .map_err(Into::into)
}

#[get("/api/devin_status")]
pub async fn get_devin_status() -> Result<DevinStatus> {
    Ok(DevinStatus {
        configured: devin::configured(),
        budget_used: db::budget_used_today().await.unwrap_or(0),
        budget_total: db::budget_limit().await.unwrap_or(0),
        api_base: env::devin_api_base(),
    })
}

#[cfg(feature = "server")]
async fn release_overview_row(repo: &str) -> anyhow::Result<ReleaseOverviewRow> {
    let releases = db::list_releases(repo, 6).await?;
    let stable = db::stable_release_dates(repo, 6).await?;
    let latest_stable = releases.iter().find(|r| !r.is_prerelease);
    let prerelease_tag = releases
        .first()
        .filter(|r| r.is_prerelease)
        .map(|r| r.tag.clone());
    let days_since = latest_stable
        .and_then(|r| r.published_at.as_deref())
        .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
        .map(|t| (chrono::Utc::now() - t.with_timezone(&chrono::Utc)).num_days())
        .unwrap_or(0);
    let since = latest_stable
        .and_then(|r| r.published_at.clone())
        .unwrap_or_else(|| (chrono::Utc::now() - chrono::Duration::days(180)).to_rfc3339());
    let targets = db::list_release_targets(Some(repo)).await?;
    Ok(ReleaseOverviewRow {
        repo: repo.to_string(),
        latest_tag: latest_stable.map(|r| r.tag.clone()).unwrap_or_default(),
        latest_at: latest_stable.and_then(|r| r.published_at.clone()),
        days_since,
        prerelease_tag,
        unreleased_merged: db::merged_count_since(repo, &since).await?,
        cadence_days: cadence_days(&stable),
        next_milestone: db::list_milestones(Some(repo)).await?.into_iter().next(),
        must_ship_open: targets.iter().filter(|t| !t.done).count() as i64,
        must_ship_total: targets.len() as i64,
    })
}

#[get("/api/releases/overview")]
pub async fn get_releases_overview() -> Result<Vec<ReleaseOverviewRow>> {
    let mut rows = Vec::new();
    for repo in db::monitored_repos().await? {
        rows.push(release_overview_row(&repo).await?);
    }
    rows.sort_by_key(|r| std::cmp::Reverse(r.days_since));
    Ok(rows)
}

#[post("/api/releases/detail")]
pub async fn get_release_detail(repo: String) -> Result<ReleaseDetail> {
    let releases = db::list_releases(&repo, 10).await?;
    let since = db::latest_stable_release(&repo)
        .await?
        .map(|(_, at)| at)
        .or_else(|| Some((chrono::Utc::now() - chrono::Duration::days(180)).to_rfc3339()));
    let mut groups: std::collections::BTreeMap<&'static str, Vec<PrRow>> =
        std::collections::BTreeMap::new();
    if let Some(s) = &since {
        for p in db::merged_prs_since(&repo, s).await? {
            groups
                .entry(changelog_group(&p.into_row()))
                .or_default()
                .push(p.into_row());
        }
    }
    let unreleased: Vec<ChangelogGroup> = CHANGELOG_GROUP_ORDER
        .iter()
        .filter_map(|label| {
            groups.get(label).map(|prs| ChangelogGroup {
                label: label.to_string(),
                prs: prs.clone(),
            })
        })
        .collect();
    Ok(ReleaseDetail {
        repo: repo.clone(),
        releases,
        unreleased,
        milestones: db::list_milestones(Some(&repo)).await?,
        targets: db::list_release_targets(Some(&repo)).await?,
        since,
    })
}

#[post("/api/release_targets/add")]
pub async fn add_release_target(
    repo: String,
    version: String,
    kind: String,
    number: Option<i64>,
    title: String,
) -> Result<()> {
    auth::require_admin()?;
    let kind = kind
        .parse::<TargetKind>()
        .map_err(|e: anyhow::Error| ServerFnError::new(e.to_string()))?;
    db::add_release_target(&repo, &version, kind.as_str(), number, title)
        .await
        .map_err(Into::into)
}

#[post("/api/release_targets/remove")]
pub async fn remove_release_target(id: i64) -> Result<()> {
    auth::require_admin()?;
    db::remove_release_target(id).await.map_err(Into::into)
}

#[post("/api/release_targets/done")]
pub async fn set_release_target_done(id: i64, done: bool) -> Result<()> {
    auth::require_admin()?;
    db::set_release_target_done(id, done)
        .await
        .map_err(Into::into)
}
