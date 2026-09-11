use dioxus::prelude::*;

use super::widgets::format_compact;
use crate::api;
use crate::model::RepoRow;
use crate::Route;

#[component]
pub fn Repos() -> Element {
    let repos = use_resource(|| async move { api::list_repos().await });
    rsx! {
        div { class: "page",
            h1 { "Repos" }
            match repos() {
                Some(Ok(rows)) => rsx! {
                    div { class: "grid cards",
                        for r in rows {
                            RepoCard { key: "{r.name}", repo: r }
                        }
                    }
                },
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
