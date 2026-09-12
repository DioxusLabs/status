use dioxus::prelude::*;

use super::cache::use_cached;
use super::storage::use_debounced;
use super::widgets::date_part;
use crate::api;
use crate::components::input::Input;
use crate::components::select::{Select, SelectOption};
use crate::model::{IssueFilter, IssueRow};

#[component]
pub fn Issues() -> Element {
    let mut query = use_signal(String::new);
    let committed = use_debounced(query.into(), 300);
    let mut sort = use_signal(|| "updated".to_string());

    let filter = use_memo(move || IssueFilter {
        query: committed(),
        state: Some("open".into()),
        sort: Some(sort()),
        limit: 200,
        ..Default::default()
    });
    let issues = use_cached(
        move || {
            format!(
                "issues:{}",
                serde_json::to_string(&filter()).unwrap_or_default()
            )
        },
        move || {
            let filter = filter();
            async move { api::list_issues(filter).await }
        },
    )?;

    rsx! {
        div { class: "page",
            h1 {
                "Issues"
                if (issues.loading)() { span { class: "loading-dot", " syncing…" } }
            }
            if let Some(e) = (issues.error)() {
                p { class: "muted", "error: {e}" }
            }
            div { class: "filter-bar",
                Input {
                    class: "search",
                    placeholder: "repo:x author:y + free text",
                    value: "{query}",
                    oninput: move |e: FormEvent| query.set(e.value()),
                }
                Select {
                    default_value: "updated".to_string(),
                    on_value_change: move |v: Option<String>| {
                        if let Some(v) = v {
                            sort.set(v);
                        }
                    },
                    for (i, (v, label)) in [
                        ("updated", "Sort: recent activity"),
                        ("oldest", "Sort: oldest"),
                        ("comments", "Sort: most comments"),
                        ("reactions", "Sort: most reactions"),
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
            match (issues.value)() {
                Some(rows) => rsx! {
                    table { class: "data",
                        thead {
                            tr {
                                th { "repo" }
                                th { "#" }
                                th { "title" }
                                th { "author" }
                                th { "comments" }
                                th { "reactions" }
                                th { "updated" }
                            }
                        }
                        tbody {
                            for i in rows {
                                IssueRowEl { key: "{i.id}", issue: i }
                            }
                        }
                    }
                },
                None => rsx! { p { class: "muted", "loading…" } },
            }
        }
    }
}

#[component]
fn IssueRowEl(issue: IssueRow) -> Element {
    let updated = date_part(issue.updated_at.as_deref()).to_string();
    rsx! {
        tr { class: "row",
            td { class: "mono", "{issue.repo}" }
            td { class: "mono", "#{issue.number}" }
            td { class: "title",
                a { href: "{issue.url}", target: "_blank", "{issue.title}" }
                for l in issue.labels.iter().take(4) {
                    span { class: "label", "{l}" }
                }
            }
            td {
                span {
                    class: "author {super::widgets::assoc_label(&issue.author_association)}",
                    title: "{super::widgets::assoc_label(&issue.author_association)}",
                    "{issue.author}"
                }
            }
            td { class: "mono", "{issue.comments}" }
            td { class: "mono", "{issue.reactions}" }
            td { class: "mono muted", "{updated}" }
        }
    }
}
