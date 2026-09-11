use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct RepoRow {
    pub name: String,
    pub owner: String,
    pub description: String,
    pub stars: i64,
    pub forks: i64,
    pub open_issues: i64,
    pub open_prs: i64,
    pub pushed_at: Option<String>,
    pub is_fork: bool,
    pub monitored: bool,
    pub default_branch: String,
    pub synced_at: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct PrRow {
    pub id: i64,
    pub repo: String,
    pub number: i64,
    pub title: String,
    pub author: String,
    pub author_association: String,
    pub url: String,
    pub state: String,
    pub is_draft: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub last_activity_at: Option<String>,
    pub last_activity_by: String,
    pub additions: i64,
    pub deletions: i64,
    pub changed_files: i64,
    pub mergeable: String,
    pub review_decision: String,
    pub ci_state: String,
    pub labels: Vec<String>,
    pub reviewers: Vec<String>,
    pub unresolved_threads: i64,
    pub comments: i64,
    pub linked_issues: Vec<i64>,
    pub score: i64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct PrDetail {
    pub pr: PrRow,
    pub body: String,
    pub head_sha: String,
    pub base_ref: String,
    pub files: Vec<String>,
    pub score_parts: Vec<ScorePart>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ScorePart {
    pub label: String,
    pub points: i32,
    pub note: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct IssueRow {
    pub id: i64,
    pub repo: String,
    pub number: i64,
    pub title: String,
    pub author: String,
    pub author_association: String,
    pub url: String,
    pub state: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub labels: Vec<String>,
    pub comments: i64,
    pub reactions: i64,
    pub assignees: Vec<String>,
    pub last_activity_at: Option<String>,
    pub last_activity_by: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct CrateRow {
    pub name: String,
    pub repo: String,
    pub total_downloads: i64,
    pub recent_downloads: i64,
    pub latest_version: String,
    pub latest_version_at: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ReleaseRow {
    pub repo: String,
    pub tag: String,
    pub name: String,
    pub published_at: Option<String>,
    pub url: String,
    pub is_prerelease: bool,
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ReleaseAsset {
    pub name: String,
    pub download_count: i64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct SyncLogRow {
    pub id: i64,
    pub kind: String,
    pub repo: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub ok: bool,
    pub message: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct PrSummary {
    pub repo: String,
    pub number: i64,
    pub title: String,
    pub url: String,
    pub author: String,
    pub age_days: i64,
    pub score: i64,
    pub ci_state: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Overview {
    pub monitored_repos: i64,
    pub open_prs: i64,
    pub open_issues: i64,
    pub total_stars: i64,
    pub downloads_7d: i64,
    pub stars_7d_delta: Option<i64>,
    pub ready_to_merge: Vec<PrSummary>,
    pub quick_wins: Vec<PrSummary>,
    pub going_stale: Vec<PrSummary>,
    pub waiting_on_maintainer: Vec<PrSummary>,
    pub new_this_week: Vec<PrSummary>,
    pub first_time_contributors: Vec<PrSummary>,
    pub last_sync: Option<SyncLogRow>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct PrFilter {
    pub repos: Vec<String>,
    pub query: String,
    pub state: Option<String>,
    pub draft: Option<bool>,
    pub ci: Option<String>,
    pub review: Option<String>,
    pub mergeable: Option<bool>,
    pub association: Option<String>,
    pub size: Option<String>,
    pub min_age_days: Option<i64>,
    pub stale_days: Option<i64>,
    pub first_timers_only: bool,
    pub sort: PrSort,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrSort {
    #[default]
    Score,
    Attention,
    Newest,
    Oldest,
    LastActivity,
    Size,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct IssueFilter {
    pub repos: Vec<String>,
    pub query: String,
    pub state: Option<String>,
    pub sort: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct RepoDetail {
    pub repo: RepoRow,
    pub open_prs: i64,
    pub open_issues: i64,
    pub crates: Vec<CrateRow>,
    pub releases: Vec<ReleaseRow>,
    pub stars_trend: Vec<(String, i64)>,
    pub recent_prs: Vec<PrRow>,
    pub recent_issues: Vec<IssueRow>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct CrateSeries {
    pub name: String,
    pub daily: Vec<(String, i64)>,
    pub total: i64,
    pub recent: i64,
    pub latest_version: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Health {
    pub crates: Vec<CrateSeries>,
    pub stars_trend: Vec<(String, Vec<(String, i64)>)>,
    pub open_prs: i64,
    pub open_issues: i64,
    pub prs_merged_30d: i64,
    pub issues_closed_30d: i64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct SettingsView {
    pub is_admin: bool,
    pub repos: Vec<RepoRow>,
    pub crates: Vec<String>,
    pub sync_log: Vec<SyncLogRow>,
}
