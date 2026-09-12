use dioxus::prelude::*;

use super::widgets::{format_compact, PrSummaryList};
use crate::api;

#[component]
pub fn Overview() -> Element {
    let overview = use_resource(|| async move { api::get_overview().await });

    let Some(Ok(o)) = overview() else {
        return rsx! {
            div { class: "page",
                h1 { "Overview" }
                match overview() {
                    Some(Err(e)) => rsx! { p { class: "muted", "error: {e}" } },
                    _ => rsx! { p { class: "muted", "loading…" } },
                }
            }
        };
    };

    rsx! {
        div { class: "page",
            h1 { "Overview" }
            div { class: "health-strip",
                Stat { label: "Open PRs", value: "{o.open_prs}" }
                Stat { label: "Open issues", value: "{o.open_issues}" }
                Stat { label: "Stars (monitored)", value: format_compact(o.total_stars),
                    delta: o.stars_7d_delta.map(|d| format!("{d:+} in 7d")) }
                Stat { label: "Downloads (7d)", value: format_compact(o.downloads_7d) }
                Stat { label: "Monitored repos", value: "{o.monitored_repos}" }
            }
            if let Some(sync) = &o.last_sync {
                p { class: "muted last-sync",
                    "last sync: {sync.finished_at.clone().unwrap_or_default()} — {sync.message}"
                }
            }
            div { class: "grid two",
                DevinActivity {}
                PrSummaryList { title: "Ready to merge", items: o.ready_to_merge.clone(), empty: "nothing ready" }
                PrSummaryList { title: "Waiting on maintainer", items: o.waiting_on_maintainer.clone(), empty: "nothing waiting" }
                PrSummaryList { title: "Quick wins", items: o.quick_wins.clone(), empty: "no quick wins" }
                PrSummaryList { title: "Going stale", items: o.going_stale.clone(), empty: "nothing stale" }
                PrSummaryList { title: "New this week", items: o.new_this_week.clone(), empty: "nothing new" }
                PrSummaryList { title: "First-time contributors", items: o.first_time_contributors.clone(), empty: "none this week" }
            }
        }
    }
}

#[component]
fn DevinActivity() -> Element {
    let sessions = use_resource(|| async move { api::list_sessions(None, None, 500).await });
    let Some(Ok(rows)) = sessions() else {
        return rsx! {};
    };
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let today_n = rows
        .iter()
        .filter(|s| s.created_at.starts_with(&today))
        .count();
    let working = rows.iter().filter(|s| s.status == "working").count();
    let blocked = rows.iter().filter(|s| s.status == "blocked").count();
    rsx! {
        div { class: "card",
            h3 {
                "Devin activity "
                a { class: "muted small", href: "/bots", "→ /bots" }
            }
            p { class: "mono", "{today_n} today · {working} working · {blocked} blocked" }
        }
    }
}

#[component]
fn Stat(label: &'static str, value: String, delta: Option<String>) -> Element {
    rsx! {
        div { class: "stat",
            div { class: "stat-value", "{value}" }
            div { class: "stat-label muted",
                "{label}"
                if let Some(d) = delta {
                    span { class: "stat-delta", " {d}" }
                }
            }
        }
    }
}
