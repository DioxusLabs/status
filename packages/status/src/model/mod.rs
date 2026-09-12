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
    pub devin_configured: bool,
    pub devin_budget_used: i64,
    pub devin_budget_total: i64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct DevinSessionRow {
    pub id: i64,
    pub session_id: String,
    pub url: String,
    pub kind: String,
    pub repo: String,
    pub number: i64,
    pub head_sha: String,
    pub title: String,
    pub prompt: String,
    pub status: String,
    pub result_pr_url: String,
    pub acus_consumed: Option<f64>,
    pub error: String,
    pub created_at: String,
    pub updated_at: String,
    pub last_polled_at: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct AssessmentView {
    pub repo: String,
    pub number: i64,
    pub head_sha: String,
    pub session_id: String,
    pub verdict: String,
    pub summary: String,
    pub risks: Vec<String>,
    pub suggestions: Vec<String>,
    pub quality_score: i64,
    pub created_at: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct MilestoneRow {
    pub repo: String,
    pub number: i64,
    pub title: String,
    pub due_on: Option<String>,
    pub open_issues: i64,
    pub closed_issues: i64,
    pub url: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ReleaseTarget {
    pub id: i64,
    pub repo: String,
    pub version: String,
    pub kind: String,
    pub number: Option<i64>,
    pub title: String,
    pub done: bool,
    pub created_at: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ReleaseOverviewRow {
    pub repo: String,
    pub latest_tag: String,
    pub latest_at: Option<String>,
    pub days_since: i64,
    pub prerelease_tag: Option<String>,
    pub unreleased_merged: i64,
    pub cadence_days: Option<i64>,
    pub next_milestone: Option<MilestoneRow>,
    pub must_ship_open: i64,
    pub must_ship_total: i64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ChangelogGroup {
    pub label: String,
    pub prs: Vec<PrRow>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ReleaseDetail {
    pub repo: String,
    pub releases: Vec<ReleaseRow>,
    pub unreleased: Vec<ChangelogGroup>,
    pub milestones: Vec<MilestoneRow>,
    pub targets: Vec<ReleaseTarget>,
    pub since: Option<String>,
}

/// Changelog bucket for a merged PR: labels first, then conventional-commit
/// prefixes on the title, then author kind.
#[cfg(feature = "server")]
pub fn changelog_group(p: &PrRow) -> &'static str {
    let has_label = |needles: &[&str]| {
        p.labels.iter().any(|l| {
            let l = l.to_lowercase();
            needles.iter().any(|n| l.contains(n))
        })
    };
    let title = p.title.to_lowercase();
    let starts = |pats: &[&str]| pats.iter().any(|x| title.starts_with(x));

    if has_label(&["breaking"]) {
        "Breaking"
    } else if starts(&["feat"]) || has_label(&["feature", "enhancement"]) {
        "Features"
    } else if starts(&["fix"]) || has_label(&["bug"]) {
        "Fixes"
    } else if starts(&["docs"]) || has_label(&["docs", "documentation"]) {
        "Docs"
    } else if has_label(&["ci", "chore", "deps", "dependencies"])
        || p.author.ends_with("[bot]")
        || p.author_association == "BOT"
    {
        "Internal"
    } else {
        "Other"
    }
}

#[cfg(feature = "server")]
pub const CHANGELOG_GROUP_ORDER: &[&str] =
    &["Breaking", "Features", "Fixes", "Docs", "Other", "Internal"];

/// Median gap (days) between consecutive published_at timestamps (any order).
pub fn cadence_days(dates: &[String]) -> Option<i64> {
    let mut ts: Vec<i64> = dates
        .iter()
        .filter_map(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
        .map(|t| t.timestamp())
        .collect();
    ts.sort_unstable();
    let mut gaps: Vec<i64> = ts
        .windows(2)
        .map(|w| (w[1] - w[0]).div_euclid(86_400))
        .collect();
    if gaps.is_empty() {
        return None;
    }
    gaps.sort_unstable();
    Some(gaps[gaps.len() / 2])
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct DevinStatus {
    pub configured: bool,
    pub budget_used: i64,
    pub budget_total: i64,
    pub api_base: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pr(title: &str, labels: &[&str], author: &str, assoc: &str) -> PrRow {
        PrRow {
            title: title.into(),
            labels: labels.iter().map(|s| s.to_string()).collect(),
            author: author.into(),
            author_association: assoc.into(),
            ..Default::default()
        }
    }

    #[test]
    fn changelog_groups() {
        assert_eq!(
            changelog_group(&pr("x", &["breaking-change"], "a", "NONE")),
            "Breaking"
        );
        assert_eq!(
            changelog_group(&pr("feat: thing", &[], "a", "NONE")),
            "Features"
        );
        assert_eq!(
            changelog_group(&pr("x", &["enhancement"], "a", "NONE")),
            "Features"
        );
        assert_eq!(
            changelog_group(&pr("fix: crash", &[], "a", "NONE")),
            "Fixes"
        );
        assert_eq!(changelog_group(&pr("x", &["bug"], "a", "NONE")), "Fixes");
        assert_eq!(
            changelog_group(&pr("docs: readme", &[], "a", "NONE")),
            "Docs"
        );
        assert_eq!(
            changelog_group(&pr("x", &["documentation"], "a", "NONE")),
            "Docs"
        );
        assert_eq!(
            changelog_group(&pr("x", &["dependencies"], "a", "NONE")),
            "Internal"
        );
        assert_eq!(
            changelog_group(&pr("x", &["chore"], "a", "NONE")),
            "Internal"
        );
        assert_eq!(
            changelog_group(&pr("x", &[], "dependabot[bot]", "NONE")),
            "Internal"
        );
        assert_eq!(changelog_group(&pr("x", &[], "a", "BOT")), "Internal");
        assert_eq!(
            changelog_group(&pr("misc tweak", &[], "a", "CONTRIBUTOR")),
            "Other"
        );
        // breaking label wins over feat prefix
        assert_eq!(
            changelog_group(&pr("feat: api", &["breaking"], "a", "NONE")),
            "Breaking"
        );
    }

    #[test]
    fn cadence_median() {
        assert_eq!(cadence_days(&[]), None);
        assert_eq!(cadence_days(&["2025-01-01T00:00:00Z".into()]), None);
        let dates: Vec<String> = [
            "2025-01-01T00:00:00Z",
            "2025-01-11T00:00:00Z", // +10
            "2025-02-01T00:00:00Z", // +21
            "2025-02-11T00:00:00Z", // +10
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        assert_eq!(cadence_days(&dates), Some(10)); // gaps [10,21,10] -> median 10
        let mut rev = dates.clone();
        rev.reverse();
        assert_eq!(cadence_days(&rev), Some(10)); // order-independent
    }
}
