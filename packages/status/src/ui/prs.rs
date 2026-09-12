use dioxus::prelude::*;
use dioxus_primitives::checkbox::CheckboxState;

use super::layout::AdminState;
use super::widgets::{assoc_label, CiDot, ScoreBadge};
use crate::api;
use crate::components::checkbox::Checkbox;
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
    let mut committed = use_signal(String::new);
    let mut sort = use_signal(|| PrSort::Score);
    let mut repos_sel = use_signal(Vec::<String>::new);
    let expanded = use_signal(|| Option::<(String, i64)>::None);
    let detail = use_signal(|| Option::<crate::model::PrDetail>::None);
    let syncing = use_signal(|| false);
    let last_rows = use_signal(Vec::<PrRow>::new);
    let fetch_err = use_signal(|| Option::<String>::None);
    #[cfg_attr(not(feature = "web"), allow(unused_mut))]
    let mut columns = use_signal(default_columns);
    let mut colmenu_open = use_signal(|| Some(false));
    #[cfg(feature = "web")]
    let mut cols_loaded = use_signal(|| false);

    let repos = use_resource(|| async move { api::list_repos().await.unwrap_or_default() });

    // Persist column selection (web only).
    #[cfg(feature = "web")]
    {
        use_hook(move || {
            spawn(async move {
                if let Ok(v) = document::eval("return localStorage.getItem('prs.columns') ?? ''")
                    .await
                    .map_err(|e| e.to_string())
                    .map(|v| v.as_str().unwrap_or_default().to_string())
                {
                    if let Some(saved) = (!v.is_empty())
                        .then(|| serde_json::from_str::<Vec<String>>(&v).ok())
                        .flatten()
                    {
                        let saved: Vec<String> = saved
                            .into_iter()
                            .filter(|c| PR_COLUMNS.iter().any(|(id, _)| id == c))
                            .collect();
                        if saved.iter().any(|c| c == "title") {
                            columns.set(saved);
                        }
                    }
                }
                cols_loaded.set(true);
            });
        });
        use_effect(move || {
            let cols = columns();
            if !cols_loaded() {
                return;
            }
            document::eval(&format!(
                "localStorage.setItem('prs.columns', '{}')",
                serde_json::to_string(&cols).unwrap_or_default()
            ));
        });
    }

    // Debounce the free-text query and reflect filters in the URL.
    let pending = use_hook(|| std::rc::Rc::new(std::cell::Cell::new(0u32)));
    use_effect(move || {
        let q = query();
        let rev = pending.get() + 1;
        pending.set(rev);
        let pending = pending.clone();
        spawn(async move {
            super::sleep_ms(300).await;
            if pending.get() != rev {
                return;
            }
            if committed() != q {
                committed.set(q.clone());
            }
            #[cfg(feature = "web")]
            {
                let qs = if q.is_empty() {
                    String::new()
                } else {
                    format!("?q={}", urlencode(&q))
                };
                document::eval(&format!("history.replaceState(null, '', '/prs{qs}')"));
            }
        });
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
                        let q = urldecode(q);
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
                fallback: move |_| rsx! {
                    if last_rows().is_empty() {
                        p { class: "muted",
                            if let Some(e) = fetch_err() {
                                "error: {e}"
                            } else {
                                "loading…"
                            }
                        }
                    } else {
                        PrsTableBody {
                            rows: last_rows(),
                            expanded: expanded,
                            detail: detail,
                            columns: columns,
                        }
                    }
                },
                PrsTable {
                    committed: committed,
                    repos_sel: repos_sel,
                    sort: sort,
                    expanded: expanded,
                    detail: detail,
                    syncing: syncing,
                    last_rows: last_rows,
                    fetch_err: fetch_err,
                    columns: columns,
                }
            }
        }
    }
}

/// Fetches PRs; reports them into `last_rows` so the table (and the suspense
/// fallback during refetch) always has rows to render.
#[component]
fn PrsTable(
    committed: Signal<String>,
    repos_sel: Signal<Vec<String>>,
    sort: Signal<PrSort>,
    expanded: Signal<Option<(String, i64)>>,
    detail: Signal<Option<crate::model::PrDetail>>,
    mut syncing: Signal<bool>,
    mut last_rows: Signal<Vec<PrRow>>,
    mut fetch_err: Signal<Option<String>>,
    columns: Signal<Vec<String>>,
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

    // Sync results into last_rows during render so SSR output (and the
    // suspense fallback during refetch) always has the latest rows.
    let latest = prs.value().read().clone();
    if let Some(Ok(rows)) = &latest {
        if *last_rows.peek() != *rows {
            last_rows.set(rows.clone());
        }
    }
    use_effect(move || {
        syncing.set(matches!(*prs.state().read(), UseResourceState::Pending));
        if let Some(Err(e)) = &*prs.value().read() {
            fetch_err.set(Some(e.to_string()));
        } else {
            fetch_err.set(None);
        }
    });

    rsx! {
        if let Some(e) = fetch_err() {
            p { class: "muted", "error: {e}" }
        }
        PrsTableBody {
            rows: last_rows(),
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
    let age = pr
        .created_at
        .as_deref()
        .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
        .map(|t| (chrono::Utc::now() - t.with_timezone(&chrono::Utc)).num_days())
        .unwrap_or(0);
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

/// One row in the Columns dropdown — a Checkbox + label that toggles a column
/// without closing the menu.
#[component]
fn ColItem(id: &'static str, label: &'static str, mut columns: Signal<Vec<String>>) -> Element {
    let state = use_memo(move || {
        Some(if columns().iter().any(|c| c.as_str() == id) {
            CheckboxState::Checked
        } else {
            CheckboxState::Unchecked
        })
    });
    let fixed = id == "title";
    rsx! {
        label { class: "colrow",
            Checkbox {
                checked: state,
                disabled: fixed,
                on_checked_change: move |s: CheckboxState| {
                    let mut cur = columns();
                    if s == CheckboxState::Checked {
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
                },
            }
            "{label}"
        }
    }
}
