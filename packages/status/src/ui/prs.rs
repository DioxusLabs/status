use dioxus::prelude::*;

use super::cache::use_cached;
use super::layout::AdminState;
use super::storage::{use_debounced, use_persisted_signal_checked};
use super::widgets::{assoc_label, CiDot, MenuCheckbox, ScoreBadge};
use crate::api;
use crate::components::dropdown_menu::{DropdownMenu, DropdownMenuContent, DropdownMenuTrigger};
use crate::components::input::Input;
use crate::components::select::{Select, SelectOption};
use crate::model::{PrFilter, PrRow, PrSort};

const PR_COLUMNS: &[(&str, &str)] = &[
    ("score", "score"),
    ("repo", "repo"),
    ("number", "#"),
    ("title", "title"),
    ("author", "author"),
    ("ci", "ci"),
    ("review", "review"),
    ("size", "size"),
    ("age", "age"),
    ("last_by", "last by"),
];

fn default_columns() -> Vec<String> {
    PR_COLUMNS
        .iter()
        .filter(|(id, _)| *id != "number" && *id != "last_by")
        .map(|(id, _)| id.to_string())
        .collect()
}

#[component]
pub fn PullRequests() -> Element {
    let mut query = use_signal(String::new);
    let mut committed = use_debounced(query.into(), 300);
    let mut sort = use_signal(|| PrSort::Score);
    let mut repos_sel = use_signal(Vec::<String>::new);
    let expanded = use_signal(|| Option::<(String, i64)>::None);
    let detail = use_signal(|| Option::<crate::model::PrDetail>::None);
    let syncing = use_signal(|| false);
    let columns = use_signal(default_columns);
    let mut colmenu_open = use_signal(|| Some(false));

    let repos = use_cached(|| "repos".to_string(), || async { api::list_repos().await })?;

    // Persist column selection (web only), keeping only known ids and
    // requiring "title" to survive sanitizing.
    use_persisted_signal_checked("prs.columns", columns, |cols: Vec<String>| {
        let cols: Vec<String> = cols
            .into_iter()
            .filter(|c| PR_COLUMNS.iter().any(|(id, _)| id == c))
            .collect();
        cols.iter().any(|c| c == "title").then_some(cols)
    });

    // Reflect the committed query in the URL (client only).
    #[cfg(feature = "web")]
    use_effect(move || {
        let q = committed();
        let qs = if q.is_empty() {
            String::new()
        } else {
            format!("?q={}", urlencoding::encode(&q))
        };
        document::eval(&format!("history.replaceState(null, '', '/prs{qs}')"));
    });

    // Initial query from the URL (client only).
    use_hook(move || {
        spawn(async move {
            #[cfg(feature = "web")]
            if let Some(v) = document::eval("return window.location.search")
                .await
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
            {
                for pair in v.trim_start_matches('?').split('&') {
                    if let Some(q) = pair.strip_prefix("q=") {
                        let q = urlencoding::decode(q).unwrap_or_default().into_owned();
                        query.set(q.clone());
                        if committed() != q {
                            committed.set(q);
                        }
                    }
                }
            }
        });
    });

    let mut apply_chip = move |chip: &str| {
        let q = match chip {
            "ready" => "is:green is:approved",
            "needs-review" => "is:green",
            "conflicts" => "is:conflict",
            "drafts" => "is:draft",
            "stale" => "is:stale",
            "first-timers" => "is:first-timer",
            _ => "",
        };
        query.set(q.to_string());
        committed.set(q.to_string());
    };

    rsx! {
        div { class: "page",
            h1 { "Pull Requests" }
            div { class: "filter-bar",
                Input {
                    class: "search",
                    placeholder: "repo:x author:y label:z is:draft is:green is:approved is:conflict is:stale is:first-timer -label:z",
                    title: "GitHub-like tokens: repo:x author:y label:z -label:z is:draft is:green is:red is:approved is:conflict is:stale is:first-timer plus free text",
                    value: "{query}",
                    oninput: move |e: FormEvent| query.set(e.value()),
                    onkeydown: move |e: KeyboardEvent| {
                        if e.key() == Key::Enter {
                            committed.set(query());
                        }
                    },
                }
                Select {
                    default_value: "score".to_string(),
                    on_value_change: move |v: Option<String>| {
                        let s = match v.as_deref() {
                            Some("attention") => PrSort::Attention,
                            Some("newest") => PrSort::Newest,
                            Some("oldest") => PrSort::Oldest,
                            Some("activity") => PrSort::LastActivity,
                            Some("size") => PrSort::Size,
                            _ => PrSort::Score,
                        };
                        sort.set(s);
                    },
                    for (i, (v, label)) in [
                        ("score", "Sort: score"),
                        ("attention", "Sort: attention"),
                        ("newest", "Sort: newest"),
                        ("oldest", "Sort: oldest"),
                        ("activity", "Sort: last activity"),
                        ("size", "Sort: size"),
                    ]
                    .iter()
                    .enumerate()
                    {
                        SelectOption::<String> {
                            key: "{v}",
                            value: v.to_string(),
                            text_value: label.to_string(),
                            index: i,
                            "{label}"
                        }
                    }
                }
                DropdownMenu {
                    open: colmenu_open,
                    on_open_change: move |v: bool| colmenu_open.set(Some(v)),
                    DropdownMenuTrigger { "Columns ▾" }
                    DropdownMenuContent {
                        class: "columns-menu",
                        for (id, label) in PR_COLUMNS {
                            ColItem {
                                key: "{id}",
                                id: *id,
                                label: *label,
                                columns: columns,
                            }
                        }
                    }
                }
                if syncing() {
                    span { class: "loading-dot", "syncing…" }
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
            if let Some(e) = (repos.error)() {
                p { class: "muted", "error: {e}" }
            }
            if let Some(repos) = (repos.value)() {
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
                    syncing: syncing,
                    columns: columns,
                }
            }
        }
    }
}

/// Fetches PRs with stale-while-revalidate behavior: cached rows remain
/// visible while a changed filter is fetched in the background.
#[component]
fn PrsTable(
    committed: Signal<String>,
    repos_sel: Signal<Vec<String>>,
    sort: Signal<PrSort>,
    expanded: Signal<Option<(String, i64)>>,
    detail: Signal<Option<crate::model::PrDetail>>,
    mut syncing: Signal<bool>,
    columns: Signal<Vec<String>>,
) -> Element {
    let filter = use_memo(move || PrFilter {
        repos: repos_sel(),
        query: committed(),
        state: Some("open".into()),
        sort: sort(),
        limit: 200,
        ..Default::default()
    });
    let prs = use_cached(
        move || {
            format!(
                "prs:{}",
                serde_json::to_string(&filter()).unwrap_or_default()
            )
        },
        move || {
            let filter = filter();
            async move { api::list_prs(filter).await }
        },
    )?;

    if *syncing.peek() != (prs.loading)() {
        syncing.set((prs.loading)());
    }
    let rows = (prs.value)().unwrap_or_default();

    rsx! {
        if let Some(e) = (prs.error)() {
            p { class: "muted", "error: {e}" }
        }
        PrsTableBody {
            rows: rows,
            expanded: expanded,
            detail: detail,
            columns: columns,
        }
    }
}

#[component]
fn PrsTableBody(
    rows: Vec<PrRow>,
    expanded: Signal<Option<(String, i64)>>,
    detail: Signal<Option<crate::model::PrDetail>>,
    columns: Signal<Vec<String>>,
) -> Element {
    let admin = (consume_context::<AdminState>().0)();
    let cols = columns();
    let colspan = cols.len() + admin as usize;

    rsx! {
        table { class: "data",
            thead {
                tr {
                    for (id, label) in PR_COLUMNS
                        .iter()
                        .filter(|(id, _)| cols.iter().any(|c| c.as_str() == *id))
                    {
                        th { key: "{id}", "{label}" }
                    }
                    if admin {
                        th { "devin" }
                    }
                }
            }
            tbody {
                for p in &rows {
                    PrRowEl {
                        key: "{p.id}",
                        pr: p.clone(),
                        expanded: expanded,
                        detail: detail,
                        columns: columns,
                        admin: admin,
                        colspan: colspan,
                    }
                }
            }
        }
    }
}

fn open_pr_row(
    mut expanded: Signal<Option<(String, i64)>>,
    mut detail: Signal<Option<crate::model::PrDetail>>,
    repo: String,
    number: i64,
) {
    expanded.set(Some((repo.clone(), number)));
    spawn(async move {
        if let Ok(d) = api::get_pr(repo, number).await {
            detail.set(Some(d));
        }
    });
}

#[component]
fn PrRowEl(
    pr: PrRow,
    mut expanded: Signal<Option<(String, i64)>>,
    mut detail: Signal<Option<crate::model::PrDetail>>,
    columns: Signal<Vec<String>>,
    admin: bool,
    colspan: usize,
) -> Element {
    let is_open = expanded() == Some((pr.repo.clone(), pr.number));
    let age = crate::model::days_since_rfc3339(pr.created_at.as_deref());
    let mut assess_err = use_signal(|| Option::<String>::None);
    let mut assessing = use_signal(|| false);
    let repo = pr.repo.clone();
    let number = pr.number;
    let repo_assess = pr.repo.clone();

    let cols = columns();
    let vis = |id: &str| cols.iter().any(|c| c.as_str() == id);

    rsx! {
        tr {
            class: if is_open { "row open" } else { "row" },
            onclick: move |_| {
                if is_open {
                    expanded.set(None);
                    detail.set(None);
                } else {
                    open_pr_row(expanded, detail, repo.clone(), number);
                }
            },
            if vis("score") {
                td { ScoreBadge { score: pr.score } }
            }
            if vis("repo") {
                td { class: "mono", "{pr.repo}" }
            }
            if vis("number") {
                td { class: "mono", "#{pr.number}" }
            }
            if vis("title") {
                td { class: "title",
                    a { href: "{pr.url}", target: "_blank", onclick: move |e| e.stop_propagation(), "{pr.title}" }
                    for l in pr.labels.iter().take(4) {
                        span { class: "label", "{l}" }
                    }
                }
            }
            if vis("author") {
                td {
                    span {
                        class: "author {assoc_label(&pr.author_association)}",
                        title: "{assoc_label(&pr.author_association)}",
                        "{pr.author}"
                    }
                }
            }
            if vis("ci") {
                td { CiDot { state: pr.ci_state.clone() } }
            }
            if vis("review") {
                td { class: "mono", "{review_label(&pr.review_decision)}" }
            }
            if vis("size") {
                td { class: "mono", "+{pr.additions}/-{pr.deletions}" }
            }
            if vis("age") {
                td { class: "mono", "{age}d" }
            }
            if vis("last_by") {
                td { class: "mono", "{pr.last_activity_by}" }
            }
            if admin {
                td {
                    if assessing() {
                        span { class: "loading-dot", "…" }
                    } else {
                        button {
                            class: if assess_err().is_some() { "btn-sm err" } else { "btn-sm" },
                            title: assess_err()
                                .unwrap_or_else(|| "Run a Devin assessment for this PR".into()),
                            onclick: move |e| {
                                e.stop_propagation();
                                assessing.set(true);
                                assess_err.set(None);
                                let repo = repo_assess.clone();
                                spawn(async move {
                                    match api::assess_pr(repo.clone(), number, false).await {
                                        Ok(_) => open_pr_row(expanded, detail, repo, number),
                                        Err(e) => assess_err.set(Some(e.to_string())),
                                    }
                                    assessing.set(false);
                                });
                            },
                            if assess_err().is_some() {
                                "assess failed"
                            } else {
                                "Assess"
                            }
                        }
                    }
                }
            }
        }
        if is_open {
            tr { class: "detail-row",
                td { colspan: "{colspan}",
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

fn review_label(r: &str) -> &'static str {
    match r {
        "APPROVED" => "approved",
        "CHANGES_REQUESTED" => "changes",
        "REVIEW_REQUIRED" => "required",
        _ => "—",
    }
}

/// One row in the Columns dropdown — a Checkbox + label that toggles a column
/// without closing the menu.
#[component]
fn ColItem(id: &'static str, label: &'static str, mut columns: Signal<Vec<String>>) -> Element {
    let checked = use_memo(move || columns().iter().any(|c| c.as_str() == id));
    let fixed = id == "title";
    let toggle = EventHandler::new(move |on: bool| {
        let mut cur = columns();
        if on {
            if !cur.iter().any(|c| c.as_str() == id) {
                cur = PR_COLUMNS
                    .iter()
                    .map(|(cid, _)| cid.to_string())
                    .filter(|cid| cid == id || cur.contains(cid))
                    .collect();
            }
        } else {
            cur.retain(|c| c != id);
        }
        columns.set(cur);
    });
    rsx! {
        div {
            class: "colrow",
            onclick: move |_| {
                if !fixed {
                    toggle.call(!checked());
                }
            },
            MenuCheckbox {
                checked: checked(),
                disabled: fixed,
                on_change: toggle,
            }
            "{label}"
        }
    }
}
