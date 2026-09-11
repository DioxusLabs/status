use std::sync::OnceLock;

use anyhow::Context;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{FromRow, SqlitePool};

use crate::model::*;

static POOL: OnceLock<SqlitePool> = OnceLock::new();

pub async fn init() -> anyhow::Result<SqlitePool> {
    let dir = crate::backend::env::data_dir();
    std::fs::create_dir_all(&dir).context("create data dir")?;
    let db_path = dir.join("status.db");
    let opts = SqliteConnectOptions::new()
        .filename(&db_path)
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);
    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(opts)
        .await
        .context("connect sqlite")?;
    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .context("run migrations")?;
    let _ = POOL.set(pool.clone());
    Ok(pool)
}

pub fn pool() -> &'static SqlitePool {
    POOL.get().expect("db not initialized")
}

fn json_vec<T: serde::de::DeserializeOwned>(s: &str) -> Vec<T> {
    serde_json::from_str(s).unwrap_or_default()
}

#[derive(FromRow)]
pub(crate) struct DbRepo {
    pub name: String,
    pub owner: Option<String>,
    pub description: Option<String>,
    pub stars: i64,
    pub forks: i64,
    pub open_issues: i64,
    pub open_prs: i64,
    pub pushed_at: Option<String>,
    pub is_fork: bool,
    pub monitored: bool,
    pub default_branch: Option<String>,
    pub synced_at: Option<String>,
}

impl From<DbRepo> for RepoRow {
    fn from(r: DbRepo) -> Self {
        RepoRow {
            name: r.name,
            owner: r.owner.unwrap_or_default(),
            description: r.description.unwrap_or_default(),
            stars: r.stars,
            forks: r.forks,
            open_issues: r.open_issues,
            open_prs: r.open_prs,
            pushed_at: r.pushed_at,
            is_fork: r.is_fork,
            monitored: r.monitored,
            default_branch: r.default_branch.unwrap_or_else(|| "main".into()),
            synced_at: r.synced_at,
        }
    }
}

#[derive(FromRow)]
pub(crate) struct DbPr {
    pub id: i64,
    pub repo: String,
    pub number: i64,
    pub title: Option<String>,
    pub body: Option<String>,
    pub author: Option<String>,
    pub author_association: Option<String>,
    pub url: Option<String>,
    pub state: Option<String>,
    pub is_draft: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub merged_at: Option<String>,
    pub closed_at: Option<String>,
    pub head_sha: Option<String>,
    pub base_ref: Option<String>,
    pub additions: i64,
    pub deletions: i64,
    pub changed_files: i64,
    pub mergeable: Option<String>,
    pub review_decision: Option<String>,
    pub ci_state: String,
    pub labels_json: String,
    pub reviewers_json: String,
    pub unresolved_threads: i64,
    pub comments: i64,
    pub last_activity_at: Option<String>,
    pub last_activity_by: Option<String>,
    pub linked_issues_json: String,
    pub files_json: String,
    pub score: i64,
    pub score_breakdown_json: String,
    pub synced_at: Option<String>,
}

impl DbPr {
    #[allow(clippy::wrong_self_convention)]
    pub fn into_row(&self) -> PrRow {
        PrRow {
            id: self.id,
            repo: self.repo.clone(),
            number: self.number,
            title: self.title.clone().unwrap_or_default(),
            author: self.author.clone().unwrap_or_default(),
            author_association: self.author_association.clone().unwrap_or_default(),
            url: self.url.clone().unwrap_or_default(),
            state: self.state.clone().unwrap_or_default(),
            is_draft: self.is_draft,
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            last_activity_at: self.last_activity_at.clone(),
            last_activity_by: self.last_activity_by.clone().unwrap_or_default(),
            additions: self.additions,
            deletions: self.deletions,
            changed_files: self.changed_files,
            mergeable: self.mergeable.clone().unwrap_or_default(),
            review_decision: self.review_decision.clone().unwrap_or_default(),
            ci_state: self.ci_state.clone(),
            labels: json_vec(&self.labels_json),
            reviewers: json_vec(&self.reviewers_json),
            unresolved_threads: self.unresolved_threads,
            comments: self.comments,
            linked_issues: json_vec(&self.linked_issues_json),
            score: self.score,
        }
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn into_detail(self) -> PrDetail {
        PrDetail {
            pr: self.into_row(),
            body: self.body.clone().unwrap_or_default(),
            head_sha: self.head_sha.clone().unwrap_or_default(),
            base_ref: self.base_ref.clone().unwrap_or_default(),
            files: json_vec(&self.files_json),
            score_parts: json_vec(&self.score_breakdown_json),
        }
    }
}

#[derive(FromRow)]
pub(crate) struct DbIssue {
    pub id: i64,
    pub repo: String,
    pub number: i64,
    pub title: Option<String>,
    pub body: Option<String>,
    pub author: Option<String>,
    pub author_association: Option<String>,
    pub url: Option<String>,
    pub state: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub closed_at: Option<String>,
    pub labels_json: String,
    pub comments: i64,
    pub reactions: i64,
    pub assignees_json: String,
    pub last_activity_at: Option<String>,
    pub last_activity_by: Option<String>,
    pub synced_at: Option<String>,
}

impl From<DbIssue> for IssueRow {
    fn from(i: DbIssue) -> Self {
        IssueRow {
            id: i.id,
            repo: i.repo,
            number: i.number,
            title: i.title.unwrap_or_default(),
            author: i.author.unwrap_or_default(),
            author_association: i.author_association.unwrap_or_default(),
            url: i.url.unwrap_or_default(),
            state: i.state.unwrap_or_default(),
            created_at: i.created_at,
            updated_at: i.updated_at,
            labels: json_vec(&i.labels_json),
            comments: i.comments,
            reactions: i.reactions,
            assignees: json_vec(&i.assignees_json),
            last_activity_at: i.last_activity_at,
            last_activity_by: i.last_activity_by.unwrap_or_default(),
        }
    }
}

pub async fn upsert_pr(p: &DbPr) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO pull_requests (id, repo, number, title, body, author, author_association, url,
            state, is_draft, created_at, updated_at, merged_at, closed_at, head_sha, base_ref,
            additions, deletions, changed_files, mergeable, review_decision, ci_state, labels_json,
            reviewers_json, unresolved_threads, comments, last_activity_at, last_activity_by,
            linked_issues_json, files_json, score, score_breakdown_json, synced_at)
         VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
         ON CONFLICT(id) DO UPDATE SET repo=excluded.repo, number=excluded.number, title=excluded.title,
            body=excluded.body, author=excluded.author, author_association=excluded.author_association,
            url=excluded.url, state=excluded.state, is_draft=excluded.is_draft, created_at=excluded.created_at,
            updated_at=excluded.updated_at, merged_at=excluded.merged_at, closed_at=excluded.closed_at,
            head_sha=excluded.head_sha, base_ref=excluded.base_ref, additions=excluded.additions,
            deletions=excluded.deletions, changed_files=excluded.changed_files, mergeable=excluded.mergeable,
            review_decision=excluded.review_decision, ci_state=excluded.ci_state, labels_json=excluded.labels_json,
            reviewers_json=excluded.reviewers_json, unresolved_threads=excluded.unresolved_threads,
            comments=excluded.comments, last_activity_at=excluded.last_activity_at,
            last_activity_by=excluded.last_activity_by, linked_issues_json=excluded.linked_issues_json,
            files_json=excluded.files_json, score=excluded.score, score_breakdown_json=excluded.score_breakdown_json,
            synced_at=excluded.synced_at",
    )
    .bind(p.id)
    .bind(&p.repo)
    .bind(p.number)
    .bind(&p.title)
    .bind(&p.body)
    .bind(&p.author)
    .bind(&p.author_association)
    .bind(&p.url)
    .bind(&p.state)
    .bind(p.is_draft)
    .bind(&p.created_at)
    .bind(&p.updated_at)
    .bind(&p.merged_at)
    .bind(&p.closed_at)
    .bind(&p.head_sha)
    .bind(&p.base_ref)
    .bind(p.additions)
    .bind(p.deletions)
    .bind(p.changed_files)
    .bind(&p.mergeable)
    .bind(&p.review_decision)
    .bind(&p.ci_state)
    .bind(&p.labels_json)
    .bind(&p.reviewers_json)
    .bind(p.unresolved_threads)
    .bind(p.comments)
    .bind(&p.last_activity_at)
    .bind(&p.last_activity_by)
    .bind(&p.linked_issues_json)
    .bind(&p.files_json)
    .bind(p.score)
    .bind(&p.score_breakdown_json)
    .bind(&p.synced_at)
    .execute(pool())
    .await?;
    Ok(())
}

pub async fn upsert_issue(i: &DbIssue) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO issues (id, repo, number, title, body, author, author_association, url, state,
            created_at, updated_at, closed_at, labels_json, comments, reactions, assignees_json,
            last_activity_at, last_activity_by, synced_at)
         VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
         ON CONFLICT(id) DO UPDATE SET repo=excluded.repo, number=excluded.number, title=excluded.title,
            body=excluded.body, author=excluded.author, author_association=excluded.author_association,
            url=excluded.url, state=excluded.state, created_at=excluded.created_at, updated_at=excluded.updated_at,
            closed_at=excluded.closed_at, labels_json=excluded.labels_json, comments=excluded.comments,
            reactions=excluded.reactions, assignees_json=excluded.assignees_json,
            last_activity_at=excluded.last_activity_at, last_activity_by=excluded.last_activity_by,
            synced_at=excluded.synced_at",
    )
    .bind(i.id)
    .bind(&i.repo)
    .bind(i.number)
    .bind(&i.title)
    .bind(&i.body)
    .bind(&i.author)
    .bind(&i.author_association)
    .bind(&i.url)
    .bind(&i.state)
    .bind(&i.created_at)
    .bind(&i.updated_at)
    .bind(&i.closed_at)
    .bind(&i.labels_json)
    .bind(i.comments)
    .bind(i.reactions)
    .bind(&i.assignees_json)
    .bind(&i.last_activity_at)
    .bind(&i.last_activity_by)
    .bind(i.synced_at.as_deref().unwrap_or_default())
    .execute(pool())
    .await?;
    Ok(())
}

pub async fn upsert_repo(r: &DbRepo) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO repos (name, owner, description, stars, forks, open_issues, open_prs, pushed_at,
            is_fork, monitored, default_branch, synced_at)
         VALUES (?,?,?,?,?,?,?,?,?,?,?,?)
         ON CONFLICT(name) DO UPDATE SET owner=excluded.owner, description=excluded.description,
            stars=excluded.stars, forks=excluded.forks, open_issues=excluded.open_issues,
            open_prs=excluded.open_prs, pushed_at=excluded.pushed_at, is_fork=excluded.is_fork,
            default_branch=excluded.default_branch, synced_at=excluded.synced_at",
    )
    .bind(&r.name)
    .bind(&r.owner)
    .bind(&r.description)
    .bind(r.stars)
    .bind(r.forks)
    .bind(r.open_issues)
    .bind(r.open_prs)
    .bind(&r.pushed_at)
    .bind(r.is_fork)
    .bind(r.monitored)
    .bind(&r.default_branch)
    .bind(&r.synced_at)
    .execute(pool())
    .await?;
    Ok(())
}

pub async fn list_repos() -> anyhow::Result<Vec<RepoRow>> {
    let rows = sqlx::query_as::<_, DbRepo>("SELECT * FROM repos ORDER BY name")
        .fetch_all(pool())
        .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn monitored_repos() -> anyhow::Result<Vec<String>> {
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT name FROM repos WHERE monitored = 1 ORDER BY name")
            .fetch_all(pool())
            .await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

pub async fn get_repo(name: &str) -> anyhow::Result<Option<RepoRow>> {
    let row = sqlx::query_as::<_, DbRepo>("SELECT * FROM repos WHERE name = ?")
        .bind(name)
        .fetch_optional(pool())
        .await?;
    Ok(row.map(Into::into))
}

pub async fn get_pr(repo: &str, number: i64) -> anyhow::Result<Option<PrDetail>> {
    let row =
        sqlx::query_as::<_, DbPr>("SELECT * FROM pull_requests WHERE repo = ? AND number = ?")
            .bind(repo)
            .bind(number)
            .fetch_optional(pool())
            .await?;
    Ok(row.map(|r| r.into_detail()))
}

pub async fn open_pr_numbers(repo: &str) -> anyhow::Result<Vec<i64>> {
    let rows: Vec<(i64,)> =
        sqlx::query_as("SELECT number FROM pull_requests WHERE repo = ? AND state = 'open'")
            .bind(repo)
            .fetch_all(pool())
            .await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

pub async fn mark_pr_state(repo: &str, number: i64, state: &str) -> anyhow::Result<()> {
    sqlx::query("UPDATE pull_requests SET state = ?, closed_at = COALESCE(closed_at, ?) WHERE repo = ? AND number = ?")
        .bind(state)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(repo)
        .bind(number)
        .execute(pool())
        .await?;
    Ok(())
}

pub async fn log_sync(
    kind: &str,
    repo: &str,
    started: &str,
    ok: bool,
    message: &str,
) -> anyhow::Result<()> {
    sqlx::query("INSERT INTO sync_log (kind, repo, started_at, finished_at, ok, message) VALUES (?,?,?,?,?,?)")
        .bind(kind)
        .bind(repo)
        .bind(started)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(ok)
        .bind(message)
        .execute(pool())
        .await?;
    Ok(())
}

pub async fn last_sync() -> anyhow::Result<Option<SyncLogRow>> {
    let row =
        sqlx::query_as::<_, SyncLogDb>("SELECT * FROM sync_log ORDER BY finished_at DESC LIMIT 1")
            .fetch_optional(pool())
            .await?;
    Ok(row.map(Into::into))
}

#[derive(FromRow)]
struct SyncLogDb {
    id: i64,
    kind: Option<String>,
    repo: Option<String>,
    started_at: Option<String>,
    finished_at: Option<String>,
    ok: bool,
    message: Option<String>,
}

impl From<SyncLogDb> for SyncLogRow {
    fn from(s: SyncLogDb) -> Self {
        SyncLogRow {
            id: s.id,
            kind: s.kind.unwrap_or_default(),
            repo: s.repo.unwrap_or_default(),
            started_at: s.started_at.unwrap_or_default(),
            finished_at: s.finished_at,
            ok: s.ok,
            message: s.message.unwrap_or_default(),
        }
    }
}

pub async fn sync_log(limit: i64) -> anyhow::Result<Vec<SyncLogRow>> {
    let rows = sqlx::query_as::<_, SyncLogDb>("SELECT * FROM sync_log ORDER BY id DESC LIMIT ?")
        .bind(limit)
        .fetch_all(pool())
        .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn get_setting(key: &str) -> anyhow::Result<Option<String>> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool())
        .await?;
    Ok(row.map(|r| r.0))
}

pub async fn set_setting(key: &str, value: &str) -> anyhow::Result<()> {
    sqlx::query("INSERT INTO settings (key, value) VALUES (?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value")
        .bind(key)
        .bind(value)
        .execute(pool())
        .await?;
    Ok(())
}

pub async fn set_monitored(names: &[String]) -> anyhow::Result<()> {
    sqlx::query("UPDATE repos SET monitored = 0")
        .execute(pool())
        .await?;
    for n in names {
        sqlx::query("UPDATE repos SET monitored = 1 WHERE name = ?")
            .bind(n)
            .execute(pool())
            .await?;
    }
    Ok(())
}

pub async fn seed_monitored() -> anyhow::Result<()> {
    let defaults = [
        "dioxus",
        "blitz",
        "taffy",
        "docsite",
        "dioxus-components",
        "sdk",
        "anyrender",
        "dioxus-template",
        "dioxus-icons",
        "dioxus-code",
        "accessibility-cli",
        "subsecond",
    ];
    for n in defaults {
        sqlx::query("UPDATE repos SET monitored = 1 WHERE name = ?")
            .bind(n)
            .execute(pool())
            .await?;
    }
    Ok(())
}

pub async fn repos_empty() -> anyhow::Result<bool> {
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM repos")
        .fetch_one(pool())
        .await?;
    Ok(n == 0)
}
