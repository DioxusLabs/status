use dioxus::prelude::*;

use super::layout::AdminState;
use super::widgets::assoc_label;
use crate::api;
use crate::model::*;

fn days_since(published: Option<&str>) -> i64 {
    published
        .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
        .map(|t| (chrono::Utc::now() - t.with_timezone(&chrono::Utc)).num_days())
        .unwrap_or(0)
}

fn days_class(days: i64) -> &'static str {
    if days > 180 {
        "fail-text"
    } else if days > 90 {
        "warn-text"
    } else {
        ""
    }
}

#[component]
pub fn Releases() -> Element {
    rsx! {
        div { class: "page",
            h1 { "Releases" }
            SuspenseBoundary {
                fallback: |_| rsx! { p { class: "muted", "loading…" } },
                ReleasesBody {}
            }
        }
    }
}

#[component]
fn ReleasesBody() -> Element {
    let rows = use_server_future(api::get_releases_overview)?;
    match &*rows.value().read() {
        Some(Ok(rows)) => rsx! {
            div { class: "grid cards",
                for r in rows.iter() {
                    Link {
                        key: "{r.repo}",
                        class: "repo-card",
                        to: crate::Route::ReleaseDetailPage { repo: r.repo.clone() },
                        h3 { "{r.repo}" }
                        if r.latest_tag.is_empty() {
                            p { class: "muted", "no releases yet" }
                        } else {
                            p { class: "rel-meta",
                                span { class: "mono", "{r.latest_tag}" }
                                " · "
                                span { class: days_class(r.days_since), "{r.days_since} days since release" }
                                if let Some(pre) = &r.prerelease_tag {
                                    " · "
                                    span { class: "badge assoc", "pre {pre}" }
                                }
                            }
                        }
                        p { class: "rel-meta",
                            span { "{r.unreleased_merged} merged, unreleased" }
                            if let Some(c) = r.cadence_days {
                                " · usually every ~{c}d"
                            }
                        }
                        if let Some(m) = &r.next_milestone {
                            div { class: "rel-meta",
                                a { href: "{m.url}", target: "_blank", "milestone: {m.title}" }
                                " {m.closed_issues}/{m.closed_issues + m.open_issues}"
                            }
                            div { class: "progress",
                                div {
                                    class: "progress-fill",
                                    style: "width: {milestone_pct(m)}%",
                                }
                            }
                        }
                        if r.must_ship_total > 0 {
                            p { class: "rel-meta", "must-ship {r.must_ship_total - r.must_ship_open}/{r.must_ship_total}" }
                        }
                    }
                }
            }
        },
        Some(Err(e)) => rsx! { p { class: "muted", "error: {e}" } },
        None => rsx! { p { class: "muted", "loading…" } },
    }
}

fn milestone_pct(m: &MilestoneRow) -> i64 {
    let total = m.open_issues + m.closed_issues;
    if total == 0 {
        0
    } else {
        m.closed_issues * 100 / total
    }
}

#[component]
pub fn ReleaseDetailPage(repo: String) -> Element {
    rsx! {
        div { class: "page",
            SuspenseBoundary {
                fallback: |_| rsx! { p { class: "muted", "loading…" } },
                ReleaseDetailBody { repo: repo }
            }
        }
    }
}

fn changelog_markdown(d: &ReleaseDetail) -> String {
    let mut out = String::new();
    for g in &d.unreleased {
        out.push_str(&format!("## {}\n", g.label));
        for p in &g.prs {
            out.push_str(&format!("- {} (#{})\n", p.title, p.number));
        }
        out.push('\n');
    }
    out
}

#[component]
fn ReleaseDetailBody(repo: String) -> Element {
    let mut refresh = use_signal(|| 0u32);
    let detail = use_server_future({
        let repo = repo.clone();
        move || {
            refresh();
            let repo = repo.clone();
            async move { api::get_release_detail(repo).await }
        }
    })?;
    let admin = (consume_context::<AdminState>().0)();
    let mut copied = use_signal(|| false);
    let mut form_version = use_signal(String::new);
    let mut form_kind = use_signal(|| "pr".to_string());
    let mut form_number = use_signal(String::new);
    let mut form_title = use_signal(String::new);
    let mut form_err = use_signal(|| Option::<String>::None);

    let d = match &*detail.value().read() {
        Some(Ok(d)) => d.clone(),
        Some(Err(e)) => return rsx! { p { class: "muted", "error: {e}" } },
        None => return rsx! { p { class: "muted", "loading…" } },
    };

    let latest = d.releases.iter().find(|r| !r.is_prerelease);
    let days = latest
        .map(|r| days_since(r.published_at.as_deref()))
        .unwrap_or(0);
    let markdown = changelog_markdown(&d);
    let targets = d.targets.clone();
    let default_version = d
        .milestones
        .first()
        .map(|m| m.title.clone())
        .unwrap_or_else(|| "next".into());

    rsx! {
        h1 { "{repo}" }
        if let Some(r) = latest {
            p { class: "rel-meta",
                "latest "
                a { class: "mono", href: "{r.url}", target: "_blank", "{r.tag}" }
                " · "
                span { class: days_class(days), "{days} days since release" }
                if let Some(c) = crate::model::cadence_days(
                        &d.releases.iter()
                            .filter(|x| !x.is_prerelease)
                            .filter_map(|x| x.published_at.clone())
                            .collect::<Vec<_>>()
                    ) {
                    " · usually every ~{c}d"
                }
            }
        } else {
            p { class: "muted", "no releases yet" }
        }

        div { class: "card",
            h3 {
                "Unreleased changes"
                if let Some(r) = latest {
                    span { class: "muted", " since {r.tag}" }
                }
                if let Some(s) = &d.since {
                    span { class: "muted", " ({s})" }
                }
                button {
                    class: "btn-sm",
                    style: "float: right",
                    onclick: move |_| {
                        let md = markdown.clone();
                        copied.set(false);
                        spawn(async move {
                            let js = format!(
                                "navigator.clipboard.writeText({})",
                                serde_json::to_string(&md).unwrap_or_default()
                            );
                            if document::eval(&js).await.is_ok() {
                                copied.set(true);
                            }
                        });
                    },
                    if copied() { "copied!" } else { "Copy as markdown" }
                }
            }
            if d.unreleased.is_empty() {
                p { class: "muted", "no merged PRs since the last release" }
            } else {}
            if !d.unreleased.is_empty() {
                table { class: "data fixed",
                    colgroup {
                        col { style: "width: 90px" }
                        col {}
                        col { style: "width: 160px" }
                    }
                    for g in &d.unreleased {
                        tbody {
                            tr { class: "group",
                                th { colspan: 3, "{g.label} ({g.prs.len()})" }
                            }
                            for p in &g.prs {
                                tr { class: "row", key: "{p.id}",
                                    td { class: "mono",
                                        a { href: "{p.url}", target: "_blank", "#{p.number}" }
                                    }
                                    td { class: "title", "{p.title}" }
                                    td {
                                        span {
                                            class: "author {assoc_label(&p.author_association)}",
                                            title: "{assoc_label(&p.author_association)}",
                                            "{p.author}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if !d.milestones.is_empty() {
            div { class: "card",
                h3 { "Milestones" }
                for m in &d.milestones {
                    div { class: "rel-meta", key: "{m.number}",
                        a { href: "{m.url}", target: "_blank", "{m.title}" }
                        " {m.closed_issues}/{m.closed_issues + m.open_issues}"
                        if let Some(due) = &m.due_on {
                            span { class: "muted", " · due {due}" }
                        }
                    }
                    div { class: "progress",
                        div { class: "progress-fill", style: "width: {milestone_pct(m)}%" }
                    }
                }
            }
        }

        div { class: "card",
            h3 { "Must ship" }
            if targets.is_empty() {
                p { class: "muted", "nothing tracked" }
            }
            for t in targets {
                div { class: "rel-meta target-row", key: "{t.id}",
                    input {
                        r#type: "checkbox",
                        checked: t.done,
                        disabled: !admin,
                        onchange: move |e| {
                            let id = t.id;
                            let v = e.checked();
                            spawn(async move {
                                if api::set_release_target_done(id, v).await.is_ok() {
                                    *refresh.write() += 1;
                                }
                            });
                        },
                    }
                    span { class: "mono muted", "{t.version}" }
                    span { class: "badge assoc", "{t.kind}" }
                    if let Some(n) = t.number {
                        span { class: "mono", "#{n}" }
                    }
                    span {
                        style: if t.done { "text-decoration: line-through" } else { "" },
                        "{t.title}"
                    }
                    if admin {
                        button {
                            class: "btn-sm err",
                            onclick: move |_| {
                                let id = t.id;
                                spawn(async move {
                                    if api::remove_release_target(id).await.is_ok() {
                                        *refresh.write() += 1;
                                    }
                                });
                            },
                            "✕"
                        }
                    }
                }
            }
            if admin {
                div { class: "filter-bar", style: "margin-top: 8px",
                    input {
                        placeholder: "version",
                        value: "{form_version}",
                        oninput: move |e| form_version.set(e.value()),
                    }
                    select {
                        value: "{form_kind}",
                        onchange: move |e| form_kind.set(e.value()),
                        option { value: "pr", "pr" }
                        option { value: "issue", "issue" }
                        option { value: "note", "note" }
                    }
                    input {
                        placeholder: "#",
                        style: "width: 70px",
                        value: "{form_number}",
                        oninput: move |e| form_number.set(e.value()),
                    }
                    input {
                        placeholder: "title",
                        style: "flex: 1",
                        value: "{form_title}",
                        oninput: move |e| form_title.set(e.value()),
                    }
                    button {
                        onclick: move |_| {
                            let repo = repo.clone();
                            let version = if form_version().is_empty() {
                                default_version.clone()
                            } else {
                                form_version()
                            };
                            let kind = form_kind();
                            let number = form_number().parse::<i64>().ok();
                            let title = form_title();
                            form_err.set(None);
                            spawn(async move {
                                match api::add_release_target(repo, version, kind, number, title).await {
                                    Ok(()) => {
                                        form_title.set(String::new());
                                        form_number.set(String::new());
                                        *refresh.write() += 1;
                                    }
                                    Err(e) => form_err.set(Some(e.to_string())),
                                }
                            });
                        },
                        "Add"
                    }
                }
                if let Some(e) = form_err() {
                    p { class: "fail-text", "{e}" }
                }
            }
        }

        if !d.releases.is_empty() {
            div { class: "card",
                h3 { "Releases" }
                table { class: "data",
                    tbody {
                        for r in d.releases.iter() {
                            tr { class: "row", key: "{r.tag}",
                                td { class: "mono",
                                    a { href: "{r.url}", target: "_blank", "{r.tag}" }
                                }
                                td { "{r.name}" }
                                td {
                                    if r.is_prerelease {
                                        span { class: "badge assoc", "pre" }
                                    }
                                }
                                td { class: "mono muted", "{r.published_at.clone().unwrap_or_default()}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
