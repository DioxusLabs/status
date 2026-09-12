use dioxus::prelude::*;

use super::widgets::format_compact;
use crate::api;
use crate::model::RepoRow;
use crate::Route;

#[component]
pub fn Repos() -> Element {
    let repos = use_resource(|| async move { api::list_repos().await });
    let mut sort = use_signal(|| "pushed".to_string());
    rsx! {
        div { class: "page",
            div { class: "filter-bar",
                h1 { "Repos" }
                select {
                    class: "sort",
                    onchange: move |e| sort.set(e.value()),
                    option { value: "pushed", selected: sort() == "pushed", "Sort: recently pushed" }
                    option { value: "stars", selected: sort() == "stars", "Sort: stars" }
                    option { value: "prs", selected: sort() == "prs", "Sort: open PRs" }
                    option { value: "issues", selected: sort() == "issues", "Sort: open issues" }
                    option { value: "name", selected: sort() == "name", "Sort: name" }
                }
            }
            match repos() {
                Some(Ok(rows)) => {
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
                Some(Err(e)) => rsx! { p { class: "muted", "error: {e}" } },
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
