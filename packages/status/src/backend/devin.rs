use anyhow::{anyhow, Context};
use serde::Deserialize;
use serde_json::{json, Value};

use super::env;

/// Max chars of a Devin error body included in the error message.
const ERR_BODY_MAX: usize = 300;

fn http() -> anyhow::Result<reqwest::Client> {
    let key = env::devin_api_key().ok_or_else(|| anyhow!("DEVIN_API_KEY not set"))?;
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        format!("Bearer {key}").parse()?,
    );
    reqwest::Client::builder()
        .default_headers(headers)
        .user_agent("dioxus-status")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("build devin http client")
}

fn sessions_url() -> anyhow::Result<String> {
    let org = env::devin_org_id().ok_or_else(|| anyhow!("DEVIN_ORG_ID not set"))?;
    Ok(format!(
        "{}/v3/organizations/{org}/sessions",
        env::devin_api_base()
    ))
}

pub fn configured() -> bool {
    env::devin_api_key().is_some() && env::devin_org_id().is_some()
}

#[derive(Debug, Deserialize)]
pub struct CreatedSession {
    pub session_id: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct SessionInfo {
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub status_detail: Option<String>,
    #[serde(default)]
    pub pull_requests: Vec<PullRequest>,
    #[serde(default)]
    pub structured_output: Option<Value>,
    #[serde(default)]
    pub acus_consumed: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct PullRequest {
    pub pr_url: String,
}

/// Map Devin's v3 status + status_detail to our status column. Structured
/// output on a session that is no longer actively working counts as finished —
/// `structured_output_required` sessions stop at waiting_for_user when done.
pub fn map_status(status: &str, detail: Option<&str>, has_result: bool) -> String {
    match (status, detail) {
        (_, Some("working")) if has_result => "working",
        _ if has_result => "finished",
        ("exit", _) | (_, Some("finished")) => "finished",
        ("error", _) | (_, Some("error")) => "error",
        ("suspended", _) => "blocked",
        ("running", Some("waiting_for_user" | "waiting_for_approval")) => "blocked",
        ("new" | "claimed" | "resuming", _) => "created",
        _ => "working",
    }
    .to_string()
}

async fn check(resp: reqwest::Response) -> anyhow::Result<reqwest::Response> {
    if resp.status().is_success() {
        return Ok(resp);
    }
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    // RFC 7807 errors: {status, title, detail}
    let detail = serde_json::from_str::<Value>(&body)
        .ok()
        .and_then(|v| v["detail"].as_str().map(String::from))
        .unwrap_or_else(|| body.chars().take(ERR_BODY_MAX).collect());
    Err(anyhow!("devin api {status}: {detail}"))
}

/// POST /v3/organizations/{org}/sessions
pub async fn create_session(
    prompt: &str,
    title: &str,
    tags: &[String],
    repo: &str,
    structured_output_schema: Option<Value>,
    max_acu_limit: Option<i64>,
) -> anyhow::Result<CreatedSession> {
    let mut body = json!({
        "prompt": prompt,
        "title": title,
        "tags": tags,
        "repos": [format!("DioxusLabs/{repo}")],
    });
    if let Some(s) = structured_output_schema {
        body["structured_output_schema"] = s;
        body["structured_output_required"] = json!(true);
    }
    if let Some(m) = max_acu_limit {
        body["max_acu_limit"] = json!(m);
    }
    let resp = http()?
        .post(sessions_url()?)
        .json(&body)
        .send()
        .await
        .context("devin create_session request")?;
    check(resp)
        .await?
        .json()
        .await
        .context("devin create_session")
}

/// GET /v3/organizations/{org}/sessions/{id}
pub async fn get_session(session_id: &str) -> anyhow::Result<SessionInfo> {
    let resp = http()?
        .get(format!("{}/{session_id}", sessions_url()?))
        .send()
        .await
        .context("devin get_session request")?;
    check(resp).await?.json().await.context("devin get_session")
}

/// POST /v3/organizations/{org}/sessions/{id}/messages
pub async fn send_message(session_id: &str, message: &str) -> anyhow::Result<()> {
    let resp = http()?
        .post(format!("{}/{session_id}/messages", sessions_url()?))
        .json(&json!({ "message": message }))
        .send()
        .await
        .context("devin send_message request")?;
    check(resp).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::map_status;

    #[test]
    fn status_mapping() {
        assert_eq!(
            map_status("running", Some("waiting_for_user"), true),
            "finished"
        );
        assert_eq!(map_status("running", Some("working"), false), "working");
        assert_eq!(map_status("running", Some("working"), true), "working");
        assert_eq!(
            map_status("running", Some("waiting_for_user"), false),
            "blocked"
        );
        assert_eq!(map_status("suspended", None, false), "blocked");
        assert_eq!(map_status("exit", None, false), "finished");
        assert_eq!(map_status("error", None, false), "error");
        assert_eq!(map_status("new", None, false), "created");
    }
}
