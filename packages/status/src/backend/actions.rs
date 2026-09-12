use anyhow::{anyhow, Context};
use serde_json::{json, Value};

use crate::model::{AssessmentView, DevinSessionRow, PrDetail};

use super::{db, devin};

pub const KINDS: &[&str] = &[
    "assess",
    "rebase",
    "address_reviews",
    "deslop",
    "add_tests",
    "deep_review",
    "split",
    "changelog_entry",
    "custom",
];

/// Rules baked into every prompt.
const RULES: &str = r#"You are working in the DioxusLabs/{repo} repository.

Rules:
- Check out PR #{number} before working (gh pr checkout {number} or the equivalent).
- Never force-push to the contributor's branch. If you make changes, push them to a NEW
  branch named devin/pr-{number}-{kind} and open a pull request — target the
  contributor's PR branch if it accepts pushes (stacked PR), otherwise target master
  and say in the PR body which you chose and why.
- Do not add Co-Authored-By lines to commits.
- End with a short summary of what you did or found.

Pull request context:
- Repo: DioxusLabs/{repo}
- PR: #{number} — {title}
- URL: {url}
- Author: {author}
- Base: {base_ref}  Head: {head_sha}
- Labels: {labels}
- CI: {ci_state}  Mergeable: {mergeable}  Review decision: {review_decision}
- Unresolved review threads: {unresolved_threads}
- Body (truncated):
{body}

Changed files:
{files}

"#;

const ASSESS_BODY: &str = r#"Assess this pull request for mergeability and quality. Review the diff,
CI state, and review discussion. Do NOT write or push any code. Fill in the
structured output: verdict (merge / needs_work / reject / unsure), a quality
score 0-100, a concise summary, concrete risks, and actionable suggestions."#;

const REBASE_BODY: &str = r#"Rebase the PR's branch onto the latest {base_ref}. Resolve trivial
conflicts if you can do so safely; if conflicts are non-trivial, stop and report
what is blocking instead of guessing."#;

const ADDRESS_REVIEWS_BODY: &str = r#"Address all unresolved review comments on this PR. Make the requested
changes where they are reasonable, run the relevant tests, and push your work on
a new branch as described above."#;

const DESLOP_BODY: &str = r#"Clean up this PR: apply the `deslop-rust-skill` skill and Rust idioms.
Remove dead code, redundant comments, and AI slop. No behavior changes — the
diff should be cleanup only."#;

const ADD_TESTS_BODY: &str = r#"Add missing tests for the code changed in this PR, following the repo's
existing test conventions. No behavior changes to production code."#;

const DEEP_REVIEW_BODY: &str = r#"Do a deep code review of this PR: correctness, edge cases, performance,
API design, adherence to Dioxus conventions. Post your findings as review
comments or a summary — do not modify the code."#;

const SPLIT_BODY: &str = r#"This PR is large or does multiple things. Propose a split: identify
self-contained pieces, and if feasible create the first piece as a new focused
PR on a new branch. Explain the split plan in your summary."#;

const CHANGELOG_BODY: &str = r#"Write a changelog entry for this PR matching the repo's changelog format
and the PR's labels. If the repo has a changelog file or release-notes
convention, follow it; otherwise provide the entry text in your summary."#;

fn body_for(kind: &str) -> anyhow::Result<&'static str> {
    Ok(match kind {
        "assess" => ASSESS_BODY,
        "rebase" => REBASE_BODY,
        "address_reviews" => ADDRESS_REVIEWS_BODY,
        "deslop" => DESLOP_BODY,
        "add_tests" => ADD_TESTS_BODY,
        "deep_review" => DEEP_REVIEW_BODY,
        "split" => SPLIT_BODY,
        "changelog_entry" => CHANGELOG_BODY,
        "custom" => "",
        other => return Err(anyhow!("unknown action kind: {other}")),
    })
}

fn render(template: &str, vars: &[(&str, String)]) -> String {
    let mut out = template.to_string();
    for (k, v) in vars {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

/// Build the full prompt for `kind` on this PR.
pub fn build_prompt(kind: &str, pr: &PrDetail, custom: Option<&str>) -> anyhow::Result<String> {
    let body_kind = if kind == "custom" {
        custom
            .filter(|c| !c.trim().is_empty())
            .ok_or_else(|| anyhow!("custom action needs a prompt"))?
            .to_string()
    } else {
        body_for(kind)?.to_string()
    };
    let p = &pr.pr;
    let files = if pr.files.is_empty() {
        "(unknown)".to_string()
    } else {
        pr.files.join("\n")
    };
    let vars = [
        ("repo", p.repo.clone()),
        ("number", p.number.to_string()),
        ("kind", kind.to_string()),
        ("title", p.title.clone()),
        ("url", p.url.clone()),
        ("author", p.author.clone()),
        ("base_ref", pr.base_ref.clone()),
        ("head_sha", pr.head_sha.clone()),
        ("labels", p.labels.join(", ")),
        ("ci_state", p.ci_state.clone()),
        ("mergeable", p.mergeable.clone()),
        ("review_decision", p.review_decision.clone()),
        ("unresolved_threads", p.unresolved_threads.to_string()),
        ("body", pr.body.chars().take(4000).collect()),
        ("files", files),
    ];
    Ok(format!("{}{}", render(RULES, &vars), body_kind))
}

pub fn assess_schema() -> Value {
    json!({
        "type": "object",
        "required": ["verdict", "quality_score", "summary", "risks", "suggestions"],
        "properties": {
            "verdict": {"type": "string", "enum": ["merge", "needs_work", "reject", "unsure"]},
            "quality_score": {"type": "integer", "minimum": 0, "maximum": 100},
            "summary": {"type": "string"},
            "risks": {"type": "array", "items": {"type": "string"}},
            "suggestions": {"type": "array", "items": {"type": "string"}}
        }
    })
}

/// Check budget, create the Devin session, then record it. Nothing is stored
/// and no budget is consumed when the API call fails.
pub async fn dispatch(
    repo: &str,
    number: i64,
    kind: &str,
    custom: Option<&str>,
) -> anyhow::Result<DevinSessionRow> {
    if !KINDS.contains(&kind) {
        return Err(anyhow!("unknown action kind: {kind}"));
    }
    let pr = db::get_pr(repo, number)
        .await?
        .with_context(|| format!("PR {repo}#{number} not in database"))?;
    let prompt = build_prompt(kind, &pr, custom)?;
    db::budget_consume().await?;
    let is_assess = kind == "assess";
    let tags = vec![
        "status-dashboard".to_string(),
        kind.to_string(),
        repo.to_string(),
    ];
    let created = devin::create_session(
        &prompt,
        &format!("{repo}#{number} {kind}"),
        &tags,
        repo,
        is_assess.then(assess_schema),
        is_assess.then_some(2),
    )
    .await?;
    let id = db::insert_devin_session(&db::NewDevinSession {
        session_id: &created.session_id,
        url: &created.url,
        kind,
        repo,
        number,
        head_sha: &pr.head_sha,
        title: &pr.pr.title,
        prompt: &prompt,
    })
    .await?;
    let row = db::get_devin_session(&created.session_id)
        .await?
        .with_context(|| format!("devin_sessions row {id} missing"))?;
    Ok(row)
}

/// Dispatch an assess session unless a fresh assessment exists for the PR's
/// current head_sha. `force` bypasses the cache.
pub async fn dispatch_assess(
    repo: &str,
    number: i64,
    force: bool,
) -> anyhow::Result<AssessmentView> {
    let pr = db::get_pr(repo, number)
        .await?
        .with_context(|| format!("PR {repo}#{number} not in database"))?;
    if !force {
        if let Some(a) = db::get_assessment(repo, number, &pr.head_sha).await? {
            return Ok(a);
        }
    }
    let session = dispatch(repo, number, "assess", None).await?;
    Ok(AssessmentView {
        repo: repo.to_string(),
        number,
        head_sha: pr.head_sha,
        session_id: session.session_id,
        ..Default::default()
    })
}

/// Poll each pending session once; on finished assess sessions store the
/// structured output as a `pr_assessments` row.
pub async fn poll_session(row: &DevinSessionRow) -> anyhow::Result<()> {
    let info = devin::get_session(&row.session_id).await?;
    let result_pr = info.pull_requests.first().map(|p| p.pr_url.clone());
    let structured = info.structured_output.as_ref().map(|v| v.to_string());
    let status = devin::map_status(
        &info.status,
        info.status_detail.as_deref(),
        result_pr.is_some() || structured.is_some(),
    );
    db::update_devin_session(
        &row.session_id,
        &status,
        result_pr.as_deref(),
        structured.as_deref(),
        info.acus_consumed,
        None,
    )
    .await?;
    if status == "finished" && row.kind == "assess" {
        if let Some(out) = info.structured_output {
            db::upsert_assessment(&row.repo, row.number, &row.head_sha, &row.session_id, &out)
                .await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::PrRow;

    fn detail() -> PrDetail {
        PrDetail {
            pr: PrRow {
                repo: "dioxus".into(),
                number: 42,
                title: "Fix things".into(),
                url: "https://github.com/DioxusLabs/dioxus/pull/42".into(),
                author: "someone".into(),
                ..Default::default()
            },
            head_sha: "abc123".into(),
            base_ref: "main".into(),
            ..Default::default()
        }
    }

    #[test]
    fn assess_prompt_has_context() {
        let p = build_prompt("assess", &detail(), None).unwrap();
        assert!(p.contains("DioxusLabs/dioxus"));
        assert!(p.contains("#42"));
        assert!(p.contains("https://github.com/DioxusLabs/dioxus/pull/42"));
    }

    #[test]
    fn deslop_prompt_mentions_skill() {
        let p = build_prompt("deslop", &detail(), None).unwrap();
        assert!(p.contains("deslop-rust-skill"));
        assert!(p.contains("devin/pr-42-deslop"));
        assert!(p.contains("dioxus"));
    }

    #[test]
    fn unknown_kind_fails() {
        assert!(build_prompt("nope", &detail(), None).is_err());
        assert!(build_prompt("custom", &detail(), None).is_err());
        assert!(build_prompt("custom", &detail(), Some("do x")).is_ok());
    }
}
