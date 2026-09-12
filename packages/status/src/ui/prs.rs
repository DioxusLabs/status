use dioxus::prelude::*;

use super::widgets::{CiDot, ScoreBadge};
use crate::api;
use crate::model::{PrFilter, PrRow, PrSort};

#[component]
pub fn PullRequests() -> Element {
    let mut query = use_signal(String::new);
    let mut committed = use_signal(String::new);
    let mut sort = use_signal(|| PrSort::Score);
    let mut repos_sel = use_signal(Vec::<String>::new);
    let expanded = use_signal(|| Option::<(String, i64)>::None);
    let detail = use_signal(|| Option::<crate::model::PrDetail>::None);

    let repos = use_resource(|| async move { api::list_repos().await.unwrap_or_default() });

    // Debounce the free-text query and reflect filters in the URL.
    let pending = use_hook(|| std::rc::Rc::new(std::cell::Cell::new(0u32)));
    use_effect(move || {
        let q = query();
        let rev = pending.get() + 1;
        pending.set(rev);
        let pending = pending.clone();
        spawn(async move {
            super::sleep_ms(300).await;
            if pending.get() == rev {
                committed.set(q.clone());
                #[cfg(feature = "web")]
                {
                    let qs = if q.is_empty() {
                        String::new()
                    } else {
                        format!("?q={}", urlencode(&q))
                    };
                    document::eval(&format!("history.replaceState(null, '', '/prs{qs}')"));
                }
            }
        });
    });

    // Initial query from the URL (client only).
    use_hook(move || {
        spawn(async move {
            #[cfg(feature = "web")]
            if let Ok(v) = document::eval("return window.location.search")
                .recv::<String>()
                .await
            {
                for pair in v.trim_start_matches('?').split('&') {
                    if let Some(q) = pair.strip_prefix("q=") {
                        let q = urldecode(q);
                        query.set(q.clone());
                        committed.set(q);
                    }
                }
            }
        });
    });

    // (resource lives in PrsTable so it can suspend under the SuspenseBoundary)

    let mut apply_chip = move |chip: &str| {
        let q = match chip {
            "ready" => "is:green is:approved",
            "needs-review" => "",
            "conflicts" => "is:conflict",
            "drafts" => "is:draft",
            "stale" => "is:stale",
            "first-timers" => "is:first-timer",
            _ => "",
        };
        let q = if chip == "needs-review" {
            "is:green".to_string()
        } else {
            q.to_string()
        };
        query.set(q.clone());
        committed.set(q);
    };

    rsx! {
        div { class: "page",
            h1 { "Pull Requests" }
            div { class: "filter-bar",
                input {
                    class: "search",
                    placeholder: "repo:x author:y label:z is:draft is:green is:approved is:conflict is:stale is:first-timer -label:z",
                    title: "GitHub-like tokens: repo:x author:y label:z -label:z is:draft is:green is:red is:approved is:conflict is:stale is:first-timer plus free text",
                    value: "{query}",
                    oninput: move |e| query.set(e.value()),
                    onkeydown: move |e| {
                        if e.key() == Key::Enter {
                            committed.set(query());
                        }
                    },
                }
                select {
                    class: "sort",
                    onchange: move |e| {
                        let s = match e.value().as_str() {
                            "attention" => PrSort::Attention,
                            "newest" => PrSort::Newest,
                            "oldest" => PrSort::Oldest,
                            "activity" => PrSort::LastActivity,
                            "size" => PrSort::Size,
                            _ => PrSort::Score,
                        };
                        sort.set(s);
                    },
                    option { value: "score", selected: sort() == PrSort::Score, "Sort: score" }
                    option { value: "attention", selected: sort() == PrSort::Attention, "Sort: attention" }
                    option { value: "newest", "Sort: newest" }
                    option { value: "oldest", "Sort: oldest" }
                    option { value: "activity", "Sort: last activity" }
                    option { value: "size", "Sort: size" }
                }
            }
            div { class: "chips",
                for (id, label) in [
                    ("ready", "Ready"), ("needs-review", "Needs review"), ("conflicts", "Conflicts"),
                    ("drafts", "Drafts"), ("stale", "Stale"), ("first-timers", "First-timers"),
                ] {
                    button { class: "chip", onclick: move |_| apply_chip(id), "{label}" }
                }
            }
            if let Some(repos) = repos() {
                div { class: "chips repos",
                    for r in repos.iter().filter(|r| r.monitored) {
                        {
                            let name = r.name.clone();
                            let active = repos_sel().contains(&name);
                            rsx! {
                                button {
                                    class: if active { "chip active" } else { "chip" },
                                    onclick: move |_| {
                                        let mut cur = repos_sel();
                                        if active {
                                            cur.retain(|x| x != &name);
                                        } else {
                                            cur.push(name.clone());
                                        }
                                        repos_sel.set(cur);
                                    },
                                    "{r.name}"
                                }
                            }
                        }
                    }
                }
            }
            SuspenseBoundary {
                fallback: |_| rsx! { p { class: "muted", "loading…" } },
                PrsTable {
                    committed: committed,
                    repos_sel: repos_sel,
                    sort: sort,
                    expanded: expanded,
                    detail: detail,
                }
            }
        }
    }
}

#[component]
fn PrsTable(
    committed: Signal<String>,
    repos_sel: Signal<Vec<String>>,
    sort: Signal<PrSort>,
    expanded: Signal<Option<(String, i64)>>,
    detail: Signal<Option<crate::model::PrDetail>>,
) -> Element {
    let prs = use_server_future(move || {
        let filter = PrFilter {
            repos: repos_sel(),
            query: committed(),
            state: Some("open".into()),
            sort: sort(),
            limit: 200,
            ..Default::default()
        };
        async move { api::list_prs(filter).await }
    })?;
    let out = match &*prs.value().read() {
        Some(Ok(rows)) => rsx! {
            table { class: "data",
                thead {
                    tr {
                        th { "score" }
                        th { "repo" }
                        th { "#" }
                        th { "title" }
                        th { "author" }
                        th { "ci" }
                        th { "review" }
                        th { "size" }
                        th { "age" }
                        th { "last by" }
                    }
                }
                tbody {
                    for p in rows {
                        PrRowEl {
                            key: "{p.id}",
                            pr: p.clone(),
                            expanded: expanded,
                            detail: detail,
                        }
                    }
                }
            }
        },
        Some(Err(e)) => rsx! { p { class: "muted", "error: {e}" } },
        None => rsx! { p { class: "muted", "loading…" } },
    };
    out
}

#[component]
fn PrRowEl(
    pr: PrRow,
    mut expanded: Signal<Option<(String, i64)>>,
    mut detail: Signal<Option<crate::model::PrDetail>>,
) -> Element {
    let is_open = expanded() == Some((pr.repo.clone(), pr.number));
    let age = pr
        .created_at
        .as_deref()
        .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
        .map(|t| (chrono::Utc::now() - t.with_timezone(&chrono::Utc)).num_days())
        .unwrap_or(0);
    let repo = pr.repo.clone();
    let number = pr.number;
    rsx! {
        tr {
            class: if is_open { "row open" } else { "row" },
            onclick: move |_| {
                if is_open {
                    expanded.set(None);
                    detail.set(None);
                } else {
                    expanded.set(Some((repo.clone(), number)));
                    let repo = repo.clone();
                    spawn(async move {
                        if let Ok(d) = api::get_pr(repo, number).await {
                            detail.set(Some(d));
                        }
                    });
                }
            },
            td { ScoreBadge { score: pr.score } }
            td { class: "mono", "{pr.repo}" }
            td { class: "mono", "#{pr.number}" }
            td { class: "title",
                a { href: "{pr.url}", target: "_blank", onclick: move |e| e.stop_propagation(), "{pr.title}" }
                for l in pr.labels.iter().take(4) {
                    span { class: "label", "{l}" }
                }
            }
            td {
                "{pr.author} "
                span { class: "badge assoc", "{assoc_label(&pr.author_association)}" }
            }
            td { CiDot { state: pr.ci_state.clone() } }
            td { class: "mono", "{review_label(&pr.review_decision)}" }
            td { class: "mono", "+{pr.additions}/-{pr.deletions}" }
            td { class: "mono", "{age}d" }
            td { class: "mono", "{pr.last_activity_by}" }
        }
        if is_open {
            tr { class: "detail-row",
                td { colspan: "10",
                    match detail() {
                        Some(d) if d.pr.number == pr.number => rsx! {
                            div { class: "pr-detail",
                                div { class: "cols",
                                    div {
                                        h4 { "Score breakdown" }
                                        for part in d.score_parts {
                                            div { class: "score-part",
                                                span { class: "mono pts", "{part.points:+}" }
                                                " {part.label}: {part.note}"
                                            }
                                        }
                                    }
                                    div {
                                        h4 { "Files ({d.files.len()})" }
                                        for f in d.files.iter().take(30) {
                                            div { class: "mono file", "{f}" }
                                        }
                                        if !d.pr.linked_issues.is_empty() {
                                            h4 { "Linked issues" }
                                            for i in &d.pr.linked_issues {
                                                span { class: "mono", "#{i} " }
                                            }
                                        }
                                    }
                                }
                                super::bots::DevinPanel {
                                    repo: d.pr.repo.clone(),
                                    number: d.pr.number,
                                }
                            }
                        },
                        _ => rsx! { div { class: "muted", "loading…" } },
                    }
                }
            }
        }
    }
}

fn assoc_label(a: &str) -> &'static str {
    match a {
        "MEMBER" | "OWNER" => "maintainer",
        "CONTRIBUTOR" => "contributor",
        "FIRST_TIME_CONTRIBUTOR" | "FIRST_TIMER" => "first-timer",
        "BOT" => "bot",
        _ => "",
    }
}

fn review_label(r: &str) -> &'static str {
    match r {
        "APPROVED" => "approved",
        "CHANGES_REQUESTED" => "changes",
        "REVIEW_REQUIRED" => "required",
        _ => "—",
    }
}

#[cfg(feature = "web")]
fn urlencode(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || "-_.~".contains(c) {
                c.to_string()
            } else {
                format!("%{:02X}", c as u32)
            }
        })
        .collect()
}

#[cfg(feature = "web")]
fn urldecode(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let h: String = chars.by_ref().take(2).collect();
            if let Ok(b) = u8::from_str_radix(&h, 16) {
                out.push(b as char);
            }
        } else if c == '+' {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}
