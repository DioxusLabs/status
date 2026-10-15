use dioxus::prelude::*;

use super::cache::use_cached;
use crate::api;
use crate::components::badge::{Badge, BadgeVariant};
use crate::components::button::{Button, ButtonSize, ButtonVariant};
use crate::components::dialog::Dialog;
use crate::model::{ActionKind, AssessmentView};

#[component]
fn StatusBadge(status: String) -> Element {
    let variant = match status.as_str() {
        "finished" => BadgeVariant::Primary,
        "working" | "created" => BadgeVariant::Secondary,
        "blocked" | "expired" | "error" => BadgeVariant::Destructive,
        _ => BadgeVariant::Outline,
    };
    rsx! { Badge { variant, "{status}" } }
}

#[component]
fn VerdictBadge(verdict: String) -> Element {
    let variant = match verdict.as_str() {
        "merge" => BadgeVariant::Primary,
        "needs_work" | "unsure" => BadgeVariant::Secondary,
        "reject" => BadgeVariant::Destructive,
        _ => BadgeVariant::Outline,
    };
    rsx! { Badge { variant, "{verdict}" } }
}

/// Devin section for a PR: cached assessment (public), session history, and
/// admin action buttons.
#[component]
pub fn DevinPanel(repo: String, number: i64) -> Element {
    let mut refresh = use_signal(|| 0u32);
    let mut custom_open = use_signal(|| false);
    let mut custom_text = use_signal(String::new);

    let repo_r = repo.clone();
    let assessment = use_resource(move || {
        let repo = repo_r.clone();
        async move {
            let _ = refresh();
            api::get_assessment(repo, number).await
        }
    });
    let repo_s = repo.clone();
    let sessions = use_resource(move || {
        let repo = repo_s.clone();
        async move {
            let _ = refresh();
            api::list_sessions(Some(repo), Some(number), 20).await
        }
    });
    let admin = use_resource(|| async move { api::is_admin().await.unwrap_or(false) });

    let repo_d = repo.clone();
    let mut dispatch = use_action(move |(kind, custom): (ActionKind, Option<String>)| {
        let repo = repo_d.clone();
        async move { api::dispatch_action(repo, number, kind.to_string(), custom).await }
    });
    let repo_a = repo.clone();
    let mut assess = use_action(move |force: bool| {
        let repo = repo_a.clone();
        async move { api::assess_pr(repo, number, force).await }
    });
    use_effect(move || {
        if let Some(Ok(_)) = dispatch.value() {
            *refresh.write() += 1;
        }
        if let Some(Ok(_)) = assess.value() {
            *refresh.write() += 1;
        }
    });

    let cached = matches!(&*assessment.read(), Some(Ok(Some(_))));
    let pending = dispatch.pending() || assess.pending();

    rsx! {
        div { class: "devin-panel",
            h4 { "Devin" }
            match &*assessment.read() {
                Some(Ok(Some(a))) => rsx! { AssessmentCard { a: a.clone() } },
                Some(Err(e)) => rsx! { p { class: "fail-text", "{e}" } },
                _ => rsx! {},
            }
            match &*sessions.read() {
                Some(Ok(rows)) if !rows.is_empty() => rsx! {
                    for s in rows {
                        div { class: "devin-session", key: "{s.id}",
                            StatusBadge { status: s.status.clone() }
                            span { class: "mono", " {s.kind}" }
                            if !s.url.is_empty() {
                                a { href: "{s.url}", target: "_blank", class: "mono", " session" }
                            }
                            if !s.result_pr_url.is_empty() {
                                a { href: "{s.result_pr_url}", target: "_blank", class: "mono", " result" }
                            }
                            if !s.error.is_empty() {
                                span { class: "fail-text small", " {s.error}" }
                            }
                            span { class: "muted small", " {s.created_at}" }
                        }
                    }
                },
                Some(Err(e)) => rsx! { p { class: "fail-text", "{e}" } },
                _ => rsx! {},
            }
            match dispatch.value() {
                Some(Ok(_)) => rsx! { p { class: "muted small", "session created" } },
                Some(Err(e)) => rsx! { p { class: "fail-text", "{e}" } },
                None => rsx! {},
            }
            if let Some(Err(e)) = assess.value() {
                p { class: "fail-text", "{e}" }
            }
            if admin() == Some(true) {
                div { class: "chips",
                    Button {
                        variant: ButtonVariant::Outline,
                        size: ButtonSize::Sm,
                        disabled: cached || pending,
                        title: if cached { "cached for this head" } else { "" },
                        onclick: move |_| assess.call(false),
                        "Assess"
                    }
                    if cached {
                        Button {
                            variant: ButtonVariant::Ghost,
                            size: ButtonSize::Sm,
                            disabled: pending,
                            title: "force re-run",
                            onclick: move |_| assess.call(true),
                            "re-run"
                        }
                    }
                    for k in ActionKind::ALL.iter().filter(|k| k.is_dispatchable()) {
                        Button {
                            key: "{k}",
                            variant: ButtonVariant::Outline,
                            size: ButtonSize::Sm,
                            disabled: pending,
                            onclick: move |_| dispatch.call((*k, None)),
                            "{k.label()}"
                        }
                    }
                    Button {
                        variant: ButtonVariant::Outline,
                        size: ButtonSize::Sm,
                        disabled: pending,
                        onclick: move |_| custom_open.set(true),
                        "Custom…"
                    }
                }
                Dialog {
                    is_modal: true,
                    open: custom_open(),
                    on_open_change: move |v: bool| custom_open.set(v),
                    h3 { "Custom action" }
                    textarea {
                        class: "dialog-text",
                        rows: 6,
                        placeholder: "Instructions for the agent…",
                        value: "{custom_text}",
                        oninput: move |e| custom_text.set(e.value()),
                    }
                    div { class: "chips",
                        Button {
                            size: ButtonSize::Sm,
                            disabled: pending || custom_text().trim().is_empty(),
                            onclick: move |_| {
                                dispatch.call((ActionKind::Custom, Some(custom_text())));
                                custom_open.set(false);
                            },
                            "Dispatch"
                        }
                        Button {
                            variant: ButtonVariant::Ghost,
                            size: ButtonSize::Sm,
                            onclick: move |_| custom_open.set(false),
                            "Cancel"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn AssessmentCard(a: AssessmentView) -> Element {
    rsx! {
        div { class: "card assessment",
            div {
                VerdictBadge { verdict: a.verdict.clone() }
                span { class: "mono", " score {a.quality_score}" }
                if !a.session_id.is_empty() {
                    span { class: "muted small", " (head {a.head_sha.chars().take(7).collect::<String>()})" }
                }
            }
            p { "{a.summary}" }
            if !a.risks.is_empty() {
                ul { class: "risks",
                    for r in &a.risks {
                        li { "{r}" }
                    }
                }
            }
            if !a.suggestions.is_empty() {
                ul { class: "suggestions",
                    for s in &a.suggestions {
                        li { "{s}" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn Bots() -> Element {
    let mut refresh = use_signal(|| 0u32);
    let mut repo_filter = use_signal(String::new);
    let mut msg_for = use_signal(|| Option::<String>::None);
    let mut msg_text = use_signal(String::new);

    let status = use_cached(
        || "devin:status".to_string(),
        move || async move {
            let _ = refresh();
            api::get_devin_status().await
        },
    )?;
    let sessions = use_cached(
        move || format!("sessions:{}", repo_filter()),
        move || async move {
            let _ = refresh();
            let f = repo_filter();
            api::list_sessions(if f.is_empty() { None } else { Some(f) }, None, 200).await
        },
    )?;
    let admin = use_resource(|| async move { api::is_admin().await.unwrap_or(false) });

    let mut poll =
        use_action(move |session_id: String| async move { api::refresh_session(session_id).await });
    let mut send = use_action(move |(id, msg): (String, String)| async move {
        api::send_session_message(id, msg).await
    });
    use_effect(move || {
        if let Some(Ok(_)) = poll.value() {
            *refresh.write() += 1;
        }
    });

    rsx! {
        div { class: "page",
            h1 {
                "Bots"
                if (status.loading)() || (sessions.loading)() {
                    span { class: "loading-dot", " syncing…" }
                }
            }
            if let Some(e) = (status.error)() { p { class: "muted", "error: {e}" } }
            match (status.value)() {
                Some(s) => rsx! {
                    div { class: "card narrow",
                        h3 { "Devin" }
                        p {
                            "configured: "
                            if s.configured { span { class: "ok-text", "yes" } } else { span { class: "fail-text", "no" } }
                        }
                        p { class: "mono", "budget today: {s.budget_used}/{s.budget_total}" }
                        p { class: "muted small", "{s.api_base}" }
                    }
                },
                None => rsx! {},
            }
            if let Some(e) = (sessions.error)() { p { class: "fail-text", "{e}" } }
            div { class: "card",
                div { class: "filterbar",
                    input {
                        class: "search",
                        placeholder: "filter by repo…",
                        value: "{repo_filter}",
                        oninput: move |e| repo_filter.set(e.value()),
                    }
                }
                match (sessions.value)() {
                    Some(rows) if rows.is_empty() => rsx! { p { class: "muted", "No sessions yet." } },
                    Some(rows) => rsx! {
                        table { class: "data",
                            thead {
                                tr {
                                    th { "time" }
                                    th { "pr" }
                                    th { "kind" }
                                    th { "status" }
                                    th { "session" }
                                    th { "result" }
                                    th { "" }
                                }
                            }
                            tbody {
                                for s in rows {
                                    tr { class: "row", key: "{s.id}",
                                        td { class: "mono muted", "{s.created_at.chars().take(16).collect::<String>()}" }
                                        td { class: "mono",
                                            a {
                                                href: "https://github.com/DioxusLabs/{s.repo}/pull/{s.number}",
                                                target: "_blank",
                                                "{s.repo}#{s.number}"
                                            }
                                        }
                                        td { class: "mono", "{s.kind}" }
                                        td { StatusBadge { status: s.status.clone() } }
                                        td {
                                            if !s.url.is_empty() {
                                                a { href: "{s.url}", target: "_blank", class: "mono", "devin" }
                                            }
                                            if !s.error.is_empty() {
                                                div { class: "fail-text small", "{s.error}" }
                                            }
                                        }
                                        td {
                                            if !s.result_pr_url.is_empty() {
                                                a { href: "{s.result_pr_url}", target: "_blank", class: "mono", "pr" }
                                            }
                                        }
                                        td {
                                            if admin() == Some(true) {
                                                Button {
                                                    variant: ButtonVariant::Ghost,
                                                    size: ButtonSize::Xs,
                                                    onclick: {
                                                        let id = s.session_id.clone();
                                                        move |_| poll.call(id.clone())
                                                    },
                                                    "Refresh"
                                                }
                                                Button {
                                                    variant: ButtonVariant::Ghost,
                                                    size: ButtonSize::Xs,
                                                    onclick: {
                                                        let id = s.session_id.clone();
                                                        move |_| msg_for.set(Some(id.clone()))
                                                    },
                                                    "Message"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    None => rsx! { p { class: "muted", "loading…" } },
                }
            }
            Dialog {
                is_modal: true,
                open: msg_for().is_some(),
                on_open_change: move |v: bool| {
                    if !v {
                        msg_for.set(None);
                    }
                },
                h3 { "Message session" }
                textarea {
                    class: "dialog-text",
                    rows: 5,
                    value: "{msg_text}",
                    oninput: move |e| msg_text.set(e.value()),
                }
                if let Some(Err(e)) = send.value() {
                    p { class: "fail-text", "{e}" }
                }
                div { class: "chips",
                    Button {
                        size: ButtonSize::Sm,
                        disabled: send.pending() || msg_text().trim().is_empty(),
                        onclick: move |_| {
                            if let Some(id) = msg_for() {
                                send.call((id, msg_text()));
                            }
                            msg_for.set(None);
                            msg_text.set(String::new());
                        },
                        "Send"
                    }
                    Button {
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Sm,
                        onclick: move |_| msg_for.set(None),
                        "Cancel"
                    }
                }
            }
        }
    }
}
