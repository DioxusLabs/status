use dioxus::prelude::*;

use super::cache::use_cached;
use super::widgets::{format_compact, PrSummaryList};
use crate::api;
use crate::components::button::{Button, ButtonSize, ButtonVariant};
use crate::components::checkbox::Checkbox;
use crate::components::dropdown_menu::{DropdownMenu, DropdownMenuContent, DropdownMenuTrigger};
use dioxus_primitives::checkbox::CheckboxState;

#[component]
pub fn Overview() -> Element {
    let mut selected = use_signal(Vec::<String>::new);
    let mut menu_open = use_signal(|| Some(false));
    #[cfg(feature = "web")]
    let mut repos_loaded = use_signal(|| false);
    let repos = use_cached(|| "repos".to_string(), || async { api::list_repos().await })?;

    #[cfg(feature = "web")]
    {
        use_hook(move || {
            spawn(async move {
                if let Ok(v) = document::eval("return localStorage.getItem('overview.repos') ?? ''")
                    .await
                    .map_err(|e| e.to_string())
                    .map(|v| v.as_str().unwrap_or_default().to_string())
                {
                    if let Some(saved) = (!v.is_empty())
                        .then(|| serde_json::from_str::<Vec<String>>(&v).ok())
                        .flatten()
                    {
                        selected.set(saved);
                    }
                }
                repos_loaded.set(true);
            });
        });
        use_effect(move || {
            let repos = selected();
            if !repos_loaded() {
                return;
            }
            document::eval(&format!(
                "localStorage.setItem('overview.repos', '{}')",
                serde_json::to_string(&repos).unwrap_or_default()
            ));
        });
    }

    let overview = use_cached(
        move || {
            format!(
                "overview:{}",
                serde_json::to_string(&selected()).unwrap_or_default()
            )
        },
        move || {
            let repos = selected();
            async move { api::get_overview(repos).await }
        },
    )?;
    let mut repo_options = (repos.value)().unwrap_or_default();
    repo_options.retain(|r| r.monitored);
    repo_options.sort_by_key(|r| std::cmp::Reverse(r.stars));
    let selected_count = selected()
        .iter()
        .filter(|r| r.as_str() != "__none__")
        .count();
    let filter_label = if selected().is_empty() {
        "All repos".to_string()
    } else if selected().iter().any(|r| r == "__none__") {
        "0 repos".to_string()
    } else if selected_count == 1 {
        selected()
            .iter()
            .find(|r| r.as_str() != "__none__")
            .cloned()
            .unwrap_or_default()
    } else {
        format!("{selected_count} repos")
    };
    let Some(o) = (overview.value)() else {
        return rsx! { div { class: "page", h1 { "Overview" } } };
    };

    rsx! {
        div { class: "page",
            div { class: "page-head",
                h1 {
                    "Overview"
                    if (overview.loading)() { span { class: "loading-dot", " syncing…" } }
                }
                DropdownMenu {
                    open: menu_open,
                    on_open_change: move |v: bool| menu_open.set(Some(v)),
                    DropdownMenuTrigger { "{filter_label}" }
                    DropdownMenuContent {
                        class: "columns-menu",
                        div { class: "colrow filter-actions",
                            Button {
                                variant: ButtonVariant::Ghost,
                                size: ButtonSize::Xs,
                                onclick: move |_| selected.set(Vec::new()),
                                "select all"
                            }
                            Button {
                                variant: ButtonVariant::Ghost,
                                size: ButtonSize::Xs,
                                onclick: move |_| selected.set(vec!["__none__".to_string()]),
                                "deselect all"
                            }
                        }
                        for repo in repo_options.iter() {
                            OverviewRepoItem {
                                key: "{repo.name}",
                                name: repo.name.clone(),
                                selected: selected,
                            }
                        }
                    }
                }
            }
            if let Some(e) = (overview.error)() {
                p { class: "muted", "error: {e}" }
            }
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
fn OverviewRepoItem(name: String, mut selected: Signal<Vec<String>>) -> Element {
    let state_name = name.clone();
    let state = use_memo(move || {
        Some(
            if selected().is_empty() || selected().iter().any(|repo| repo == &state_name) {
                CheckboxState::Checked
            } else {
                CheckboxState::Unchecked
            },
        )
    });
    let label = name
        .strip_prefix("DioxusLabs/")
        .unwrap_or(&name)
        .to_string();
    rsx! {
        label { class: "colrow",
            Checkbox {
                checked: state,
                on_checked_change: move |state: CheckboxState| {
                    let mut repos = selected();
                    repos.retain(|repo| repo != "__none__");
                    if state == CheckboxState::Checked {
                        if !repos.iter().any(|repo| repo == &name) {
                            repos.push(name.clone());
                        }
                    } else {
                        repos.retain(|repo| repo != &name);
                    }
                    selected.set(repos);
                },
            }
            "{label}"
        }
    }
}

#[component]
fn DevinActivity() -> Element {
    let sessions = use_cached(
        || "sessions:all".to_string(),
        || async { api::list_sessions(None, None, 500).await },
    )?;
    let Some(rows) = (sessions.value)() else {
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
