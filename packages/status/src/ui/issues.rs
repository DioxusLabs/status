use dioxus::prelude::*;

use crate::api;
use crate::model::{IssueFilter, IssueRow};

#[component]
pub fn Issues() -> Element {
    let mut query = use_signal(String::new);
    let mut committed = use_signal(String::new);
    let mut sort = use_signal(|| "updated".to_string());

    let pending = use_hook(|| std::rc::Rc::new(std::cell::Cell::new(0u32)));
    use_effect(move || {
        let q = query();
        let rev = pending.get() + 1;
        pending.set(rev);
        let pending = pending.clone();
        spawn(async move {
            super::sleep_ms(300).await;
            if pending.get() == rev {
                committed.set(q);
            }
        });
    });

    let issues = use_resource(move || async move {
        api::list_issues(IssueFilter {
            query: committed(),
            state: Some("open".into()),
            sort: Some(sort()),
            limit: 200,
            ..Default::default()
        })
        .await
    });

    rsx! {
        div { class: "page",
            h1 { "Issues" }
            div { class: "filter-bar",
                input {
                    class: "search",
                    placeholder: "repo:x author:y + free text",
                    value: "{query}",
                    oninput: move |e| query.set(e.value()),
                }
                select {
                    class: "sort",
                    onchange: move |e| sort.set(e.value()),
                    option { value: "updated", "Sort: recent activity" }
                    option { value: "oldest", "Sort: oldest" }
                    option { value: "comments", "Sort: most comments" }
                    option { value: "reactions", "Sort: most reactions" }
                }
            }
            match issues() {
                Some(Ok(rows)) => rsx! {
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
                Some(Err(e)) => rsx! { p { class: "muted", "error: {e}" } },
                None => rsx! { p { class: "muted", "loading…" } },
            }
        }
    }
}

#[component]
fn IssueRowEl(issue: IssueRow) -> Element {
    let updated = issue
        .updated_at
        .as_deref()
        .and_then(|t| t.split('T').next())
        .unwrap_or("")
        .to_string();
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
            td { "{issue.author}" }
            td { class: "mono", "{issue.comments}" }
            td { class: "mono", "{issue.reactions}" }
            td { class: "mono muted", "{updated}" }
        }
    }
}
