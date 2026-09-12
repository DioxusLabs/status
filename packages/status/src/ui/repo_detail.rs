use dioxus::prelude::*;

use super::widgets::{format_compact, CiDot, ScoreBadge, Sparkline};
use crate::api;

#[component]
pub fn RepoDetail(name: String) -> Element {
    rsx! {
        div { class: "page",
            SuspenseBoundary {
                fallback: |_| rsx! { p { class: "muted", "loading…" } },
                RepoDetailBody { name: name }
            }
        }
    }
}

#[component]
fn RepoDetailBody(name: String) -> Element {
    let mut tab = use_signal(|| "overview".to_string());
    let detail = use_server_future({
        let name = name.clone();
        move || {
            let name = name.clone();
            async move { api::get_repo(name).await }
        }
    })?;
    let out = match &*detail.value().read() {
        Some(Ok(d)) => rsx! {
                    h1 { "{d.repo.name}" }
                    p { class: "muted", "{d.repo.description}" }
                    div { class: "health-strip",
                        div { class: "stat",
                            div { class: "stat-value", "{format_compact(d.repo.stars)}" }
                            div { class: "stat-label muted", "stars" }
                        }
                        div { class: "stat",
                            div { class: "stat-value", "{d.open_prs}" }
                            div { class: "stat-label muted", "open PRs" }
                        }
                        div { class: "stat",
                            div { class: "stat-value", "{d.open_issues}" }
                            div { class: "stat-label muted", "open issues" }
                        }
                        div { class: "stat",
                            div { class: "stat-value", "{format_compact(d.repo.forks)}" }
                            div { class: "stat-label muted", "forks" }
                        }
                    }
                    div { class: "tabs",
                        for t in ["overview", "prs", "issues", "releases", "crates"] {
                            button {
                                class: if tab() == t { "tab active" } else { "tab" },
                                onclick: move |_| tab.set(t.to_string()),
                                "{t}"
                            }
                        }
                    }
                    match tab().as_str() {
                        "prs" => rsx! {
                            table { class: "data",
                                tbody {
                                    for p in d.recent_prs.iter() {
                                        tr { class: "row", key: "{p.id}",
                                            td { ScoreBadge { score: p.score } }
                                            td { class: "mono", "#{p.number}" }
                                            td { class: "title",
                                                a { href: "{p.url}", target: "_blank", "{p.title}" }
                                            }
                                            td { "{p.author}" }
                                            td { CiDot { state: p.ci_state.clone() } }
                                        }
                                    }
                                }
                            }
                        },
                        "issues" => rsx! {
                            table { class: "data",
                                tbody {
                                    for i in d.recent_issues.iter() {
                                        tr { class: "row", key: "{i.id}",
                                            td { class: "mono", "#{i.number}" }
                                            td { class: "title",
                                                a { href: "{i.url}", target: "_blank", "{i.title}" }
                                            }
                                            td { "{i.author}" }
                                            td { class: "mono", "{i.comments}" }
                                        }
                                    }
                                }
                            }
                        },
                        "releases" => rsx! {
                            table { class: "data",
                                tbody {
                                    for r in d.releases.iter() {
                                        tr { class: "row", key: "{r.tag}",
                                            td { class: "mono",
                                                a { href: "{r.url}", target: "_blank", "{r.tag}" }
                                            }
                                            td { "{r.name}" }
                                            td { class: "mono muted", "{r.published_at.clone().unwrap_or_default()}" }
                                            td { class: "mono",
                                                "{r.assets.iter().map(|a| a.download_count).sum::<i64>()} dl"
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        "crates" => rsx! {
                            table { class: "data",
                                tbody {
                                    for c in d.crates.iter() {
                                        tr { class: "row", key: "{c.name}",
                                            td { class: "mono", "{c.name}" }
                                            td { class: "mono", "{format_compact(c.total_downloads)}" }
                                            td { class: "mono", "{format_compact(c.recent_downloads)} recent" }
                                            td { class: "mono muted", "{c.latest_version}" }
                                        }
                                    }
                                }
                            }
                        },
                        _ => rsx! {
                            div { class: "card",
                                h3 { "Stars (30d trend)" }
                                Sparkline { data: d.stars_trend.clone(), width: 480.0, height: 60.0 }
                            }
                        },
                    }
        },
        Some(Err(e)) => rsx! { p { class: "muted", "error: {e}" } },
        None => rsx! { p { class: "muted", "loading…" } },
    };
    out
}
