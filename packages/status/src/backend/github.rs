use std::collections::HashSet;
use std::time::Duration;

use anyhow::{anyhow, Context};
use serde_json::{json, Value};

use super::db::{self, DbIssue, DbPr};
use super::scoring::{self, ScoreInput};

static CLIENT: tokio::sync::OnceCell<octocrab::Octocrab> = tokio::sync::OnceCell::const_new();

/// Octocrab client. Prefers GitHub App auth (GITHUB_APP_ID + private key +
/// installation for the org); octocrab mints the RS256 JWT and refreshes the
/// installation token itself. Falls back to GITHUB_TOKEN when the app is not
/// configured or not installed.
pub async fn client() -> anyhow::Result<&'static octocrab::Octocrab> {
    CLIENT
        .get_or_try_init(|| async { build_client().await })
        .await
}

async fn build_client() -> anyhow::Result<octocrab::Octocrab> {
    if let (Some(app_id), Some(pem)) = (
        super::env::github_app_id(),
        super::env::github_app_private_key(),
    ) {
        match app_client(app_id, &pem).await {
            Ok(c) => {
                tracing::info!("github auth: app installation token (app {app_id})");
                return Ok(c);
            }
            Err(e) => {
                tracing::warn!("github app auth unavailable, using GITHUB_TOKEN: {e:#}");
            }
        }
    }
    tracing::info!("github auth: personal access token");
    let token = super::env::github_token().context("GITHUB_TOKEN not set")?;
    octocrab::Octocrab::builder()
        .personal_token(token)
        .build()
        .context("build octocrab client")
}

async fn app_client(app_id: u64, pem: &[u8]) -> anyhow::Result<octocrab::Octocrab> {
    let key =
        jsonwebtoken::EncodingKey::from_rsa_pem(pem).context("parse GITHUB_APP private key")?;
    let app = octocrab::Octocrab::builder()
        .app(octocrab::models::AppId::from(app_id), key)
        .build()
        .context("build app client")?;
    let installation_id = match super::env::github_app_installation_id() {
        Some(id) => id,
        None => {
            let org = super::env::org();
            let installs = app
                .apps()
                .installations()
                .send()
                .await
                .context("GET /app/installations")?
                .items;
            installs
                .iter()
                .find(|i| i.account.login == org)
                .map(|i| i.id.0)
                .with_context(|| format!("app not installed on org {org}"))
                .inspect_err(|e| tracing::warn!("{e:#}"))?
        }
    };
    app.installation(installation_id.into())
        .context("installation client")
}

async fn graphql(query: &str, vars: Value) -> anyhow::Result<Value> {
    let body = json!({ "query": query, "variables": vars });
    for attempt in 0..4 {
        match client().await?.graphql::<Value>(&body).await {
            Ok(resp) => {
                if let Some(errors) = resp.get("errors") {
                    let msg = errors.to_string();
                    if msg.contains("RATE_LIMITED") {
                        tracing::warn!("graphql rate limited, backing off");
                        tokio::time::sleep(Duration::from_secs(20 * (attempt + 1) as u64)).await;
                        continue;
                    }
                    return Err(anyhow!("graphql errors: {msg}"));
                }
                if let Some(remaining) = resp
                    .pointer("/data/rateLimit/remaining")
                    .and_then(|v| v.as_i64())
                {
                    if remaining < 200 {
                        tracing::warn!("github graphql remaining={remaining}");
                    }
                }
                return Ok(resp["data"].clone());
            }
            Err(e) => {
                let msg = e.to_string();
                if (msg.contains("403") || msg.contains("429") || msg.contains("secondary rate"))
                    && attempt < 3
                {
                    tracing::warn!("github error {msg}, backing off");
                    tokio::time::sleep(Duration::from_secs(30 * (attempt + 1) as u64)).await;
                    continue;
                }
                return Err(e.into());
            }
        }
    }
    Err(anyhow!("graphql retries exhausted"))
}

const ORG_MEMBERS_QUERY: &str = r#"
query($org: String!, $cursor: String) {
  rateLimit { remaining }
  organization(login: $org) {
    membersWithRole(first: 100, after: $cursor) {
      pageInfo { hasNextPage endCursor }
      nodes { login }
    }
  }
}"#;

pub async fn org_members() -> anyhow::Result<HashSet<String>> {
    let mut members = HashSet::new();
    let mut cursor: Option<String> = None;
    loop {
        let data = graphql(
            ORG_MEMBERS_QUERY,
            json!({ "org": super::env::org(), "cursor": cursor }),
        )
        .await?;
        let page = &data["organization"]["membersWithRole"];
        for n in page["nodes"].as_array().cloned().unwrap_or_default() {
            if let Some(login) = n["login"].as_str() {
                members.insert(login.to_string());
            }
        }
        if page["pageInfo"]["hasNextPage"].as_bool() == Some(true) {
            cursor = page["pageInfo"]["endCursor"].as_str().map(String::from);
        } else {
            break;
        }
    }
    Ok(members)
}

const REPOS_QUERY: &str = r#"
query($org: String!, $cursor: String) {
  rateLimit { remaining }
  organization(login: $org) {
    repositories(first: 50, after: $cursor, orderBy: {field: PUSHED_AT, direction: DESC}) {
      pageInfo { hasNextPage endCursor }
      nodes {
        name
        description
        isArchived
        isFork
        stargazerCount
        forkCount
        pushedAt
        defaultBranchRef { name }
        pullRequests(states: OPEN) { totalCount }
        issues(states: OPEN) { totalCount }
      }
    }
  }
}"#;

pub async fn list_org_repos() -> anyhow::Result<Vec<db::DbRepo>> {
    let org = super::env::org();
    let mut out = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let data = graphql(REPOS_QUERY, json!({ "org": org, "cursor": cursor })).await?;
        let page = &data["organization"]["repositories"];
        for n in page["nodes"].as_array().cloned().unwrap_or_default() {
            if n["isArchived"].as_bool() == Some(true) {
                continue;
            }
            out.push(db::DbRepo {
                name: n["name"].as_str().unwrap_or_default().to_string(),
                owner: Some(org.clone()),
                description: n["description"].as_str().map(String::from),
                stars: n["stargazerCount"].as_i64().unwrap_or(0),
                forks: n["forkCount"].as_i64().unwrap_or(0),
                open_issues: n["issues"]["totalCount"].as_i64().unwrap_or(0),
                open_prs: n["pullRequests"]["totalCount"].as_i64().unwrap_or(0),
                pushed_at: n["pushedAt"].as_str().map(String::from),
                is_fork: n["isFork"].as_bool().unwrap_or(false),
                monitored: false,
                default_branch: n["defaultBranchRef"]["name"].as_str().map(String::from),
                synced_at: Some(chrono::Utc::now().to_rfc3339()),
            });
        }
        if page["pageInfo"]["hasNextPage"].as_bool() == Some(true) {
            cursor = page["pageInfo"]["endCursor"].as_str().map(String::from);
        } else {
            break;
        }
    }
    Ok(out)
}

const PRS_QUERY: &str = r#"
query($org: String!, $repo: String!, $cursor: String) {
  rateLimit { remaining }
  repository(owner: $org, name: $repo) {
    pullRequests(states: OPEN, first: 50, after: $cursor, orderBy: {field: UPDATED_AT, direction: DESC}) {
      pageInfo { hasNextPage endCursor }
      nodes {
        databaseId
        number
        title
        body
        author { login }
        authorAssociation
        url
        isDraft
        createdAt
        updatedAt
        mergeable
        reviewDecision
        additions
        deletions
        changedFiles
        headRefOid
        baseRefName
        labels(first: 20) { nodes { name } }
        reviewRequests(first: 10) { nodes { requestedReviewer { ... on User { login } ... on Team { name } } } }
        reviewThreads(first: 50) { nodes { isResolved } }
        comments { totalCount }
        closingIssuesReferences(first: 10) { nodes { number } }
        files(first: 100) { nodes { path } }
        commits(last: 1) { nodes { commit { statusCheckRollup { state } } } }
        timelineItems(last: 1, itemTypes: [ISSUE_COMMENT, PULL_REQUEST_REVIEW, PULL_REQUEST_COMMIT]) {
          nodes {
            __typename
            ... on IssueComment { createdAt author { login } }
            ... on PullRequestReview { createdAt author { login } }
            ... on PullRequestCommit { commit { committedDate } }
          }
        }
      }
    }
  }
}"#;

fn last_activity(pr: &Value, members: &HashSet<String>) -> (Option<String>, String) {
    let mut at = pr["updatedAt"].as_str().map(String::from);
    let mut by = "contributor".to_string();
    if let Some(node) = pr["timelineItems"]["nodes"]
        .as_array()
        .and_then(|n| n.last())
    {
        let login = node["author"]["login"].as_str().unwrap_or("");
        if let Some(t) = node["createdAt"].as_str() {
            at = Some(t.to_string());
        }
        if login.ends_with("[bot]") {
            by = "bot".into();
        } else if members.contains(login) {
            by = "maintainer".into();
        }
    }
    (at, by)
}

fn ci_state(pr: &Value) -> String {
    let node = pr["commits"]["nodes"]
        .as_array()
        .and_then(|n| n.first())
        .cloned()
        .unwrap_or(Value::Null);
    match node["commit"]["statusCheckRollup"]["state"]
        .as_str()
        .unwrap_or("")
    {
        "SUCCESS" => "success".into(),
        "FAILURE" | "ERROR" => "failure".into(),
        "PENDING" | "EXPECTED" => "pending".into(),
        _ => "none".into(),
    }
}

fn map_pr(
    repo: &str,
    n: &Value,
    members: &HashSet<String>,
    open_files: &[HashSet<String>],
) -> Option<DbPr> {
    let author = n["author"]["login"].as_str().unwrap_or("").to_string();
    let association = n["authorAssociation"].as_str().unwrap_or("").to_string();
    let labels: Vec<String> = n["labels"]["nodes"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|l| l["name"].as_str().map(String::from))
        .collect();
    let files: Vec<String> = n["files"]["nodes"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|f| f["path"].as_str().map(String::from))
        .collect();
    let reviewers: Vec<String> = n["reviewRequests"]["nodes"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|r| {
            r["requestedReviewer"]["login"]
                .as_str()
                .or_else(|| r["requestedReviewer"]["name"].as_str())
                .map(String::from)
        })
        .collect();
    let linked: Vec<i64> = n["closingIssuesReferences"]["nodes"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|i| i["number"].as_i64())
        .collect();
    let unresolved = n["reviewThreads"]["nodes"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter(|t| t["isResolved"].as_bool() == Some(false))
        .count() as i64;
    let (last_activity_at, last_activity_by) = last_activity(n, members);
    let file_set: HashSet<String> = files.iter().cloned().collect();
    let overlap = open_files
        .iter()
        .filter(|o| !o.is_disjoint(&file_set))
        .count()
        > 1;
    let ci = ci_state(n);
    let days_since_activity = last_activity_at
        .as_deref()
        .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
        .map(|t| (chrono::Utc::now() - t.with_timezone(&chrono::Utc)).num_days())
        .unwrap_or(0);
    let score = scoring::score_pr(&ScoreInput {
        ci_state: ci.clone(),
        mergeable: n["mergeable"].as_str().unwrap_or("").to_string(),
        review_decision: n["reviewDecision"].as_str().unwrap_or("").to_string(),
        unresolved_threads: unresolved,
        additions: n["additions"].as_i64().unwrap_or(0),
        deletions: n["deletions"].as_i64().unwrap_or(0),
        changed_files: n["changedFiles"].as_i64().unwrap_or(0),
        files: files.clone(),
        has_description: !n["body"].as_str().unwrap_or("").trim().is_empty(),
        linked_issues: linked.clone(),
        author_association: association.clone(),
        days_since_activity,
        is_draft: n["isDraft"].as_bool().unwrap_or(false),
        labels: labels.clone(),
        file_overlap_with_open: overlap,
    });
    Some(DbPr {
        id: n["databaseId"].as_i64()?,
        repo: repo.to_string(),
        number: n["number"].as_i64()?,
        title: n["title"].as_str().map(String::from),
        body: n["body"].as_str().map(String::from),
        author: Some(author),
        author_association: Some(association),
        url: n["url"].as_str().map(String::from),
        state: Some("open".into()),
        is_draft: n["isDraft"].as_bool().unwrap_or(false),
        created_at: n["createdAt"].as_str().map(String::from),
        updated_at: n["updatedAt"].as_str().map(String::from),
        merged_at: None,
        closed_at: None,
        head_sha: n["headRefOid"].as_str().map(String::from),
        base_ref: n["baseRefName"].as_str().map(String::from),
        additions: n["additions"].as_i64().unwrap_or(0),
        deletions: n["deletions"].as_i64().unwrap_or(0),
        changed_files: n["changedFiles"].as_i64().unwrap_or(0),
        mergeable: n["mergeable"].as_str().map(String::from),
        review_decision: n["reviewDecision"].as_str().map(String::from),
        ci_state: ci,
        labels_json: serde_json::to_string(&labels).unwrap_or_else(|_| "[]".into()),
        reviewers_json: serde_json::to_string(&reviewers).unwrap_or_else(|_| "[]".into()),
        unresolved_threads: unresolved,
        comments: n["comments"]["totalCount"].as_i64().unwrap_or(0),
        last_activity_at,
        last_activity_by: Some(last_activity_by),
        linked_issues_json: serde_json::to_string(&linked).unwrap_or_else(|_| "[]".into()),
        files_json: serde_json::to_string(&files).unwrap_or_else(|_| "[]".into()),
        score: score.total as i64,
        score_breakdown_json: serde_json::to_string(&score.parts).unwrap_or_else(|_| "[]".into()),
        synced_at: Some(chrono::Utc::now().to_rfc3339()),
    })
}

/// Fetch all open PRs for a repo, upsert them, and mark any PR in the DB that is
/// no longer open upstream as closed.
pub async fn sync_open_prs(repo: &str, members: &HashSet<String>) -> anyhow::Result<usize> {
    let org = super::env::org();
    let mut cursor: Option<String> = None;
    let mut seen = HashSet::new();
    let mut raw_nodes: Vec<Value> = Vec::new();
    loop {
        let data = graphql(
            PRS_QUERY,
            json!({ "org": org, "repo": repo, "cursor": cursor }),
        )
        .await?;
        let page = &data["repository"]["pullRequests"];
        raw_nodes.extend(page["nodes"].as_array().cloned().unwrap_or_default());
        if page["pageInfo"]["hasNextPage"].as_bool() == Some(true) {
            cursor = page["pageInfo"]["endCursor"].as_str().map(String::from);
        } else {
            break;
        }
    }
    let open_files: Vec<HashSet<String>> = raw_nodes
        .iter()
        .map(|n| {
            n["files"]["nodes"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .filter_map(|f| f["path"].as_str().map(String::from))
                .collect()
        })
        .collect();
    for n in &raw_nodes {
        if let Some(pr) = map_pr(repo, n, members, &open_files) {
            seen.insert(pr.number);
            db::upsert_pr(&pr).await?;
        }
    }
    for number in db::open_pr_numbers(repo).await? {
        if !seen.contains(&number) {
            db::mark_pr_state(repo, number, "closed").await?;
        }
    }
    Ok(seen.len())
}

const CLOSED_PRS_QUERY: &str = r#"
query($org: String!, $repo: String!, $cursor: String) {
  rateLimit { remaining }
  repository(owner: $org, name: $repo) {
    pullRequests(states: [MERGED, CLOSED], first: 50, after: $cursor, orderBy: {field: UPDATED_AT, direction: DESC}) {
      pageInfo { hasNextPage endCursor }
      nodes {
        databaseId
        number
        title
        url
        state
        isDraft
        author { login }
        authorAssociation
        createdAt
        updatedAt
        mergedAt
        closedAt
        additions
        deletions
        changedFiles
        labels(first: 20) { nodes { name color } }
      }
    }
  }
}"#;

/// Fetch PRs closed/merged within the last 30 days. Uses a direct
/// `pullRequests(states: [MERGED, CLOSED])` connection ordered by updatedAt and
/// stops paging at the cutoff — cheaper than `search()` since it needs no
/// search quota and fetches exactly the fields we store.
pub async fn sync_closed_prs(repo: &str) -> anyhow::Result<usize> {
    let org = super::env::org();
    let cutoff = chrono::Utc::now() - chrono::Duration::days(30);
    let mut cursor: Option<String> = None;
    let mut count = 0;
    'pages: loop {
        let data = graphql(
            CLOSED_PRS_QUERY,
            json!({ "org": org, "repo": repo, "cursor": cursor }),
        )
        .await?;
        let page = &data["repository"]["pullRequests"];
        for n in page["nodes"].as_array().cloned().unwrap_or_default() {
            let updated = n["updatedAt"]
                .as_str()
                .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok());
            if updated.is_some_and(|t| t.with_timezone(&chrono::Utc) < cutoff) {
                break 'pages;
            }
            let state = n["state"].as_str().unwrap_or("").to_lowercase();
            let labels: Vec<String> = n["labels"]["nodes"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .filter_map(|l| l["name"].as_str().map(String::from))
                .collect();
            let pr = DbPr {
                id: n["databaseId"].as_i64().unwrap_or(0),
                repo: repo.to_string(),
                number: n["number"].as_i64().unwrap_or(0),
                title: n["title"].as_str().map(String::from),
                body: None,
                author: n["author"]["login"].as_str().map(String::from),
                author_association: n["authorAssociation"].as_str().map(String::from),
                url: n["url"].as_str().map(String::from),
                state: Some(state),
                is_draft: n["isDraft"].as_bool().unwrap_or(false),
                created_at: n["createdAt"].as_str().map(String::from),
                updated_at: n["updatedAt"].as_str().map(String::from),
                merged_at: n["mergedAt"].as_str().map(String::from),
                closed_at: n["closedAt"].as_str().map(String::from),
                head_sha: None,
                base_ref: None,
                additions: n["additions"].as_i64().unwrap_or(0),
                deletions: n["deletions"].as_i64().unwrap_or(0),
                changed_files: n["changedFiles"].as_i64().unwrap_or(0),
                mergeable: None,
                review_decision: None,
                ci_state: "none".into(),
                labels_json: serde_json::to_string(&labels).unwrap_or_else(|_| "[]".into()),
                reviewers_json: "[]".into(),
                unresolved_threads: 0,
                comments: 0,
                last_activity_at: n["updatedAt"].as_str().map(String::from),
                last_activity_by: None,
                linked_issues_json: "[]".into(),
                files_json: "[]".into(),
                score: 0,
                score_breakdown_json: "[]".into(),
                synced_at: Some(chrono::Utc::now().to_rfc3339()),
            };
            db::update_pr_lifecycle(&pr).await?;
            count += 1;
        }
        if page["pageInfo"]["hasNextPage"].as_bool() == Some(true) {
            cursor = page["pageInfo"]["endCursor"].as_str().map(String::from);
        } else {
            break;
        }
    }
    Ok(count)
}

const CLOSED_ISSUES_QUERY: &str = r#"
query($org: String!, $repo: String!, $cursor: String) {
  rateLimit { remaining }
  repository(owner: $org, name: $repo) {
    issues(states: CLOSED, first: 50, after: $cursor, orderBy: {field: UPDATED_AT, direction: DESC}) {
      pageInfo { hasNextPage endCursor }
      nodes {
        databaseId
        number
        title
        url
        state
        author { login }
        authorAssociation
        createdAt
        updatedAt
        closedAt
        labels(first: 20) { nodes { name color } }
        comments { totalCount }
        reactions { totalCount }
      }
    }
  }
}"#;

pub async fn sync_closed_issues(repo: &str) -> anyhow::Result<usize> {
    let org = super::env::org();
    let cutoff = chrono::Utc::now() - chrono::Duration::days(30);
    let mut cursor: Option<String> = None;
    let mut count = 0;
    'pages: loop {
        let data = graphql(
            CLOSED_ISSUES_QUERY,
            json!({ "org": org, "repo": repo, "cursor": cursor }),
        )
        .await?;
        let page = &data["repository"]["issues"];
        for n in page["nodes"].as_array().cloned().unwrap_or_default() {
            let updated = n["updatedAt"]
                .as_str()
                .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok());
            if updated.is_some_and(|t| t.with_timezone(&chrono::Utc) < cutoff) {
                break 'pages;
            }
            let labels: Vec<String> = n["labels"]["nodes"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .filter_map(|l| l["name"].as_str().map(String::from))
                .collect();
            db::update_issue_lifecycle(&DbIssue {
                id: n["databaseId"].as_i64().unwrap_or(0),
                repo: repo.to_string(),
                number: n["number"].as_i64().unwrap_or(0),
                title: n["title"].as_str().map(String::from),
                body: None,
                author: n["author"]["login"].as_str().map(String::from),
                author_association: n["authorAssociation"].as_str().map(String::from),
                url: n["url"].as_str().map(String::from),
                state: Some(n["state"].as_str().unwrap_or("closed").to_lowercase()),
                created_at: n["createdAt"].as_str().map(String::from),
                updated_at: n["updatedAt"].as_str().map(String::from),
                closed_at: n["closedAt"].as_str().map(String::from),
                labels_json: serde_json::to_string(&labels).unwrap_or_else(|_| "[]".into()),
                comments: n["comments"]["totalCount"].as_i64().unwrap_or(0),
                reactions: n["reactions"]["totalCount"].as_i64().unwrap_or(0),
                assignees_json: "[]".into(),
                last_activity_at: n["updatedAt"].as_str().map(String::from),
                last_activity_by: None,
                synced_at: Some(chrono::Utc::now().to_rfc3339()),
            })
            .await?;
            count += 1;
        }
        if page["pageInfo"]["hasNextPage"].as_bool() == Some(true) {
            cursor = page["pageInfo"]["endCursor"].as_str().map(String::from);
        } else {
            break;
        }
    }
    Ok(count)
}

const ISSUES_QUERY: &str = r#"
query($org: String!, $repo: String!, $cursor: String) {
  rateLimit { remaining }
  repository(owner: $org, name: $repo) {
    issues(states: OPEN, first: 50, after: $cursor, orderBy: {field: UPDATED_AT, direction: DESC}) {
      pageInfo { hasNextPage endCursor }
      nodes {
        databaseId
        number
        title
        body
        author { login }
        authorAssociation
        url
        createdAt
        updatedAt
        labels(first: 20) { nodes { name } }
        comments { totalCount }
        reactions { totalCount }
        assignees(first: 10) { nodes { login } }
        timelineItems(last: 1, itemTypes: [ISSUE_COMMENT]) {
          nodes { ... on IssueComment { createdAt author { login } } }
        }
      }
    }
  }
}"#;

pub async fn sync_open_issues(repo: &str, members: &HashSet<String>) -> anyhow::Result<usize> {
    let org = super::env::org();
    let mut cursor: Option<String> = None;
    let mut seen: HashSet<i64> = HashSet::new();
    let mut count = 0;
    loop {
        let data = graphql(
            ISSUES_QUERY,
            json!({ "org": org, "repo": repo, "cursor": cursor }),
        )
        .await?;
        let page = &data["repository"]["issues"];
        for n in page["nodes"].as_array().cloned().unwrap_or_default() {
            seen.insert(n["number"].as_i64().unwrap_or(0));
            let login = n["author"]["login"].as_str().unwrap_or("");
            let labels: Vec<String> = n["labels"]["nodes"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .filter_map(|l| l["name"].as_str().map(String::from))
                .collect();
            let assignees: Vec<String> = n["assignees"]["nodes"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .filter_map(|a| a["login"].as_str().map(String::from))
                .collect();
            let (mut at, mut by) = (
                n["updatedAt"].as_str().map(String::from),
                "contributor".to_string(),
            );
            if let Some(node) = n["timelineItems"]["nodes"]
                .as_array()
                .and_then(|v| v.last())
            {
                let alogin = node["author"]["login"].as_str().unwrap_or("");
                if let Some(t) = node["createdAt"].as_str() {
                    at = Some(t.to_string());
                }
                by = if alogin.ends_with("[bot]") {
                    "bot".into()
                } else if members.contains(alogin) {
                    "maintainer".into()
                } else {
                    "contributor".into()
                };
            }
            db::upsert_issue(&DbIssue {
                id: n["databaseId"].as_i64().unwrap_or(0),
                repo: repo.to_string(),
                number: n["number"].as_i64().unwrap_or(0),
                title: n["title"].as_str().map(String::from),
                body: n["body"].as_str().map(String::from),
                author: Some(login.to_string()),
                author_association: n["authorAssociation"].as_str().map(String::from),
                url: n["url"].as_str().map(String::from),
                state: Some("open".into()),
                created_at: n["createdAt"].as_str().map(String::from),
                updated_at: n["updatedAt"].as_str().map(String::from),
                closed_at: None,
                labels_json: serde_json::to_string(&labels).unwrap_or_else(|_| "[]".into()),
                comments: n["comments"]["totalCount"].as_i64().unwrap_or(0),
                reactions: n["reactions"]["totalCount"].as_i64().unwrap_or(0),
                assignees_json: serde_json::to_string(&assignees).unwrap_or_else(|_| "[]".into()),
                last_activity_at: at,
                last_activity_by: Some(by),
                synced_at: Some(chrono::Utc::now().to_rfc3339()),
            })
            .await?;
            count += 1;
        }
        if page["pageInfo"]["hasNextPage"].as_bool() == Some(true) {
            cursor = page["pageInfo"]["endCursor"].as_str().map(String::from);
        } else {
            break;
        }
    }
    for number in db::open_issue_numbers(repo).await? {
        if !seen.contains(&number) {
            db::mark_issue_state(repo, number, "closed").await?;
        }
    }
    Ok(count)
}

#[derive(Debug, serde::Deserialize)]
struct GhRelease {
    tag_name: String,
    name: Option<String>,
    published_at: Option<String>,
    html_url: String,
    prerelease: bool,
    #[serde(default)]
    assets: Vec<GhAsset>,
}

#[derive(Debug, serde::Deserialize)]
struct GhAsset {
    name: String,
    download_count: i64,
}

pub async fn sync_releases(repo: &str) -> anyhow::Result<usize> {
    let org = super::env::org();
    let releases: Vec<GhRelease> = client()
        .await?
        .get(
            format!("/repos/{org}/{repo}/releases?per_page=30"),
            None::<&()>,
        )
        .await?;
    let mut count = 0;
    for r in releases {
        let assets: Vec<crate::model::ReleaseAsset> = r
            .assets
            .into_iter()
            .map(|a| crate::model::ReleaseAsset {
                name: a.name,
                download_count: a.download_count,
            })
            .collect();
        sqlx::query(
            "INSERT INTO releases (repo, tag, name, published_at, url, is_prerelease, assets_json)
             VALUES (?,?,?,?,?,?,?)
             ON CONFLICT(repo, tag) DO UPDATE SET name=excluded.name, published_at=excluded.published_at,
                url=excluded.url, is_prerelease=excluded.is_prerelease, assets_json=excluded.assets_json",
        )
        .bind(repo)
        .bind(&r.tag_name)
        .bind(&r.name)
        .bind(&r.published_at)
        .bind(&r.html_url)
        .bind(r.prerelease)
        .bind(serde_json::to_string(&assets).unwrap_or_else(|_| "[]".into()))
        .execute(db::pool())
        .await?;
        count += 1;
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    #[ignore = "requires GITHUB_APP_PRIVATE_KEY_PATH + network"]
    async fn app_jwt_gets_app_slug() {
        dotenvy::from_filename("../../.env").ok();
        let pem = super::super::env::github_app_private_key().expect("no pem");
        let key = jsonwebtoken::EncodingKey::from_rsa_pem(&pem).unwrap();
        let app = octocrab::Octocrab::builder()
            .app(
                octocrab::models::AppId::from(super::super::env::github_app_id().unwrap()),
                key,
            )
            .build()
            .unwrap();
        let v: serde_json::Value = app.get("/app", None::<&()>).await.unwrap();
        assert!(v["slug"].is_string(), "{v}");
    }
}
