use dioxus::prelude::*;

use super::cache::use_cached;
use super::widgets::{format_compact, LineChart, Sparkline};
use crate::api;

/// How many crates get a downloads chart.
const TOP_CRATE_CHARTS: usize = 12;

#[component]
pub fn Health() -> Element {
    let health = use_cached(
        || "health".to_string(),
        || async { api::get_health().await },
    )?;
    let Some(h) = (health.value)() else {
        return rsx! { div { class: "page", h1 { "Health" } } };
    };

    rsx! {
        div { class: "page",
            h1 {
                "Health"
                if (health.loading)() { span { class: "loading-dot", " syncing…" } }
            }
            if let Some(e) = (health.error)() {
                p { class: "muted", "error: {e}" }
            }
            div { class: "health-strip",
                        div { class: "stat",
                            div { class: "stat-value", "{h.open_prs}" }
                            div { class: "stat-label muted", "open PRs" }
                        }
                        div { class: "stat",
                            div { class: "stat-value", "{h.open_issues}" }
                            div { class: "stat-label muted", "open issues" }
                        }
                        div { class: "stat",
                            div { class: "stat-value", "{h.prs_merged_30d}" }
                            div { class: "stat-label muted", "PRs merged (30d)" }
                        }
                        div { class: "stat",
                            div { class: "stat-value", "{h.issues_closed_30d}" }
                            div { class: "stat-label muted", "issues closed (30d)" }
                        }
                    }
                    h2 { "Downloads (90d)" }
                    div { class: "grid two",
                        for c in h.crates.iter().take(TOP_CRATE_CHARTS) {
                            div { class: "card", key: "{c.name}",
                                h3 {
                                    "{c.name} "
                                    span { class: "muted small", "v{c.latest_version} — {format_compact(c.recent)} recent" }
                                }
                                LineChart { data: c.daily.clone(), width: 560.0, height: 160.0 }
                            }
                        }
                    }
                    h2 { "All crates" }
                    table { class: "data",
                        thead {
                            tr {
                                th { "crate" }
                                th { "total downloads" }
                                th { "recent (90d)" }
                                th { "latest" }
                                th { "trend" }
                            }
                        }
                        tbody {
                            for c in h.crates.iter() {
                                tr { class: "row", key: "{c.name}",
                                    td { class: "mono", "{c.name}" }
                                    td { class: "mono", "{format_compact(c.total)}" }
                                    td { class: "mono", "{format_compact(c.recent)}" }
                                    td { class: "mono", "{c.latest_version}" }
                                    td { Sparkline { data: c.daily.clone(), width: 100.0, height: 24.0 } }
                                }
                            }
                        }
                    }
                    h2 { "Stars trend" }
                    div { class: "grid two",
                        for (repo, series) in h.stars_trend.iter() {
                            div { class: "card", key: "{repo}",
                                h3 { "{repo}" }
                                Sparkline { data: series.clone(), width: 400.0, height: 48.0 }
                            }
                        }
                    }
        }
    }
}
