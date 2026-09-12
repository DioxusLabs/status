use dioxus::prelude::*;

use super::cache::use_cached;
use super::widgets::format_compact;
use crate::api;
use crate::components::select::{Select, SelectOption};
use crate::model::RepoRow;
use crate::Route;

#[component]
pub fn Repos() -> Element {
    let repos = use_cached(|| "repos".to_string(), || async { api::list_repos().await })?;
    let mut sort = use_signal(|| "pushed".to_string());
    rsx! {
        div { class: "page",
            div { class: "filter-bar",
                h1 {
                    "Repos"
                    if (repos.loading)() { span { class: "loading-dot", " syncing…" } }
                }
                Select {
                    default_value: "pushed".to_string(),
                    on_value_change: move |v: Option<String>| {
                        sort.set(v.unwrap_or_else(|| "pushed".into()))
                    },
                    for (i, (v, label)) in [
                        ("pushed", "Sort: recently pushed"),
                        ("stars", "Sort: stars"),
                        ("prs", "Sort: open PRs"),
                        ("issues", "Sort: open issues"),
                        ("name", "Sort: name"),
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
            }
            if let Some(e) = (repos.error)() {
                p { class: "muted", "error: {e}" }
            }
            match (repos.value)() {
                Some(rows) => {
                    let mut rows = rows;
                    match sort().as_str() {
                        "stars" => rows.sort_by_key(|r| std::cmp::Reverse(r.stars)),
                        "prs" => rows.sort_by_key(|r| std::cmp::Reverse(r.open_prs)),
                        "issues" => rows.sort_by_key(|r| std::cmp::Reverse(r.open_issues)),
                        "name" => rows.sort_by(|a, b| a.name.cmp(&b.name)),
                        _ => rows.sort_by(|a, b| {
                            b.pushed_at
                                .as_deref()
                                .unwrap_or("")
                                .cmp(a.pushed_at.as_deref().unwrap_or(""))
                        }),
                    }
                    rsx! {
                        div { class: "grid cards",
                            for r in rows {
                                RepoCard { key: "{r.name}", repo: r }
                            }
                        }
                    }
                }
                None => rsx! { p { class: "muted", "loading…" } },
            }
        }
    }
}

#[component]
fn RepoCard(repo: RepoRow) -> Element {
    let pushed = repo
        .pushed_at
        .as_deref()
        .and_then(|t| t.split('T').next())
        .unwrap_or("")
        .to_string();
    rsx! {
        Link {
            class: "repo-card",
            to: Route::RepoDetail { name: repo.name.clone() },
            div { class: "repo-card-head",
                span { class: "repo-name", "{repo.name}" }
                if repo.monitored {
                    span { class: "badge assoc", "monitored" }
                }
                if repo.is_fork {
                    span { class: "badge", "fork" }
                }
            }
            p { class: "muted desc", "{repo.description}" }
            div { class: "repo-stats mono",
                span { "{format_compact(repo.stars)} ★" }
                span { "{repo.open_prs} PRs" }
                span { "{repo.open_issues} issues" }
            }
            div { class: "muted small", "pushed {pushed}" }
        }
    }
}
