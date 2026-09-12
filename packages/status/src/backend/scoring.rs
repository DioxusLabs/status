use serde::{Deserialize, Serialize};

use crate::model::{ScorePart, SizeBucket};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ScoreInput {
    pub ci_state: String,
    pub mergeable: String,
    pub review_decision: String,
    pub unresolved_threads: i64,
    pub additions: i64,
    pub deletions: i64,
    pub changed_files: i64,
    pub files: Vec<String>,
    pub has_description: bool,
    pub linked_issues: Vec<i64>,
    pub author_association: String,
    pub days_since_activity: i64,
    pub is_draft: bool,
    pub labels: Vec<String>,
    pub file_overlap_with_open: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Score {
    pub total: i32,
    pub parts: Vec<ScorePart>,
}

impl Score {
    fn add(&mut self, label: &str, points: i32, note: impl Into<String>) {
        self.parts.push(ScorePart {
            label: label.into(),
            points,
            note: note.into(),
        });
    }
}

pub fn has_tests(files: &[String]) -> bool {
    files.iter().any(|f| {
        let f = f.to_lowercase();
        f.contains("/tests/")
            || f.contains("_test")
            || f.ends_with("tests.rs")
            || f.contains("playwright")
    })
}

/// Merge-readiness score, clamped to 0..100. Weights:
///   ci +25 green / +10 pending / +0 failing / +5 none
///   mergeable +15 / 0;  review +15 approved / +5 none / -15 changes
///   threads -3 each (cap -15);  size +10/+8/+5/+2/+0 (xs..xl)
///   tests +8;  context +5;  author +5/+3/+0;  recency +5 / -10 past 90d
///   docs+src +3;  draft -30;  file overlap -5;  risky labels -15/-10
pub fn score_pr(input: &ScoreInput) -> Score {
    let mut s = Score::default();

    match input.ci_state.as_str() {
        "success" => s.add("ci", 25, "all checks green"),
        "pending" => s.add("ci", 10, "checks pending"),
        "failure" => s.add("ci", 0, "checks failing"),
        _ => s.add("ci", 5, "no checks reported"),
    }

    if input.mergeable == "MERGEABLE" || input.mergeable == "mergeable" {
        s.add("mergeable", 15, "no conflicts");
    } else if input.mergeable == "CONFLICTING" || input.mergeable == "conflicting" {
        s.add("mergeable", 0, "merge conflicts");
    } else {
        s.add("mergeable", 0, "mergeability unknown");
    }

    match input.review_decision.as_str() {
        "APPROVED" => s.add("review", 15, "approved"),
        "CHANGES_REQUESTED" => s.add("review", -15, "changes requested"),
        "REVIEW_REQUIRED" | "" => s.add("review", 5, "no review yet"),
        _ => s.add("review", 5, "no review yet"),
    }

    let threads_penalty = (input.unresolved_threads as i32 * 3).min(15);
    if threads_penalty > 0 {
        s.add(
            "threads",
            -threads_penalty,
            format!("{} unresolved threads", input.unresolved_threads),
        );
    }

    let lines = input.additions + input.deletions;
    let bucket = SizeBucket::for_lines(lines);
    let pts = match bucket {
        SizeBucket::Xs => 10,
        SizeBucket::S => 8,
        SizeBucket::M => 5,
        SizeBucket::L => 2,
        SizeBucket::Xl => 0,
    };
    s.add(
        "size",
        pts,
        format!("{bucket} ({lines} lines, {} files)", input.changed_files),
    );

    if has_tests(&input.files) {
        s.add("tests", 8, "touches tests");
    }

    if !input.linked_issues.is_empty() || input.has_description {
        s.add(
            "context",
            5,
            if input.linked_issues.is_empty() {
                "has description"
            } else {
                "links an issue"
            },
        );
    }

    match input.author_association.as_str() {
        "MEMBER" | "OWNER" => s.add("author", 5, "maintainer"),
        "CONTRIBUTOR" => s.add("author", 3, "returning contributor"),
        _ => s.add("author", 0, "first-time contributor"),
    }

    if input.days_since_activity < 7 {
        s.add("recency", 5, "active in the last week");
    } else if input.days_since_activity > 90 {
        s.add("recency", -10, "no activity in 90+ days");
    }

    let touches_docs = input
        .files
        .iter()
        .any(|f| f.ends_with(".md") || f.contains("/docs/") || f.contains("docs/"));
    let touches_src = input
        .files
        .iter()
        .any(|f| f.ends_with(".rs") && !f.contains("test"));
    if touches_docs && touches_src {
        s.add("docs", 3, "docs updated alongside code");
    }

    if input.is_draft {
        s.add("draft", -30, "marked as draft");
    }

    if input.file_overlap_with_open {
        s.add("conflict", -5, "overlaps files with another open PR");
    }

    for l in &input.labels {
        let l = l.to_lowercase();
        if l == "blocked" || l.contains("needs-design") || l == "breaking" {
            s.add("label", -15, format!("label: {l}"));
        } else if l.contains("breaking") || l.contains("blocked") {
            s.add("label", -10, format!("label: {l}"));
        }
    }

    s.total = s.parts.iter().map(|p| p.points).sum::<i32>().clamp(0, 100);
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approved_small_green_scores_high() {
        let s = score_pr(&ScoreInput {
            ci_state: "success".into(),
            mergeable: "MERGEABLE".into(),
            review_decision: "APPROVED".into(),
            additions: 10,
            deletions: 5,
            changed_files: 1,
            files: vec!["packages/core/tests/foo.rs".into()],
            has_description: true,
            linked_issues: vec![123],
            author_association: "MEMBER".into(),
            days_since_activity: 1,
            ..Default::default()
        });
        assert!(s.total >= 85, "score {} should be >= 85", s.total);
    }

    #[test]
    fn draft_with_conflicts_scores_low() {
        let s = score_pr(&ScoreInput {
            ci_state: "failure".into(),
            mergeable: "CONFLICTING".into(),
            review_decision: "CHANGES_REQUESTED".into(),
            unresolved_threads: 6,
            additions: 2000,
            deletions: 500,
            changed_files: 40,
            is_draft: true,
            days_since_activity: 100,
            labels: vec!["blocked".into()],
            ..Default::default()
        });
        assert!(s.total < 30, "score {} should be < 30", s.total);
    }

    #[test]
    fn no_review_no_ci_middling() {
        let s = score_pr(&ScoreInput {
            ci_state: "none".into(),
            mergeable: "MERGEABLE".into(),
            additions: 50,
            deletions: 10,
            changed_files: 3,
            files: vec!["src/lib.rs".into()],
            has_description: true,
            author_association: "CONTRIBUTOR".into(),
            days_since_activity: 3,
            ..Default::default()
        });
        assert!((30..=80).contains(&s.total));
    }
}
