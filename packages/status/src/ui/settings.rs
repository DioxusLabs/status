use std::collections::HashSet;

use dioxus::prelude::*;

use crate::api;

use super::layout::AdminState;

#[component]
pub fn Settings() -> Element {
    let mut admin = consume_context::<AdminState>();
    let mut token = use_signal(String::new);
    let mut error = use_signal(|| Option::<String>::None);
    let mut refresh = use_signal(|| 0u32);
    let mut monitored = use_signal(HashSet::<String>::new);
    let mut crates_text = use_signal(String::new);
    let mut budget_text = use_signal(String::new);
    let mut notice = use_signal(|| Option::<String>::None);

    let settings = use_resource(move || async move {
        let _ = refresh();
        api::get_settings().await
    });

    // Populate editable state once settings load.
    use_effect(move || {
        if let Some(Ok(s)) = settings() {
            if monitored().is_empty() {
                monitored.set(
                    s.repos
                        .iter()
                        .filter(|r| r.monitored)
                        .map(|r| r.name.clone())
                        .collect(),
                );
            }
            if crates_text().is_empty() {
                crates_text.set(s.crates.join("\n"));
            }
            if budget_text().is_empty() {
                budget_text.set(s.devin_budget_total.to_string());
            }
        }
    });

    let do_login = move || {
        let token = token();
        spawn(async move {
            match api::admin_login(token).await {
                Ok(_) => {
                    error.set(None);
                    admin.0.set(true);
                    *refresh.write() += 1;
                }
                Err(e) => error.set(Some(e.to_string())),
            }
        });
    };

    let Some(Ok(s)) = settings() else {
        return rsx! {
            div { class: "page",
                h1 { "Settings" }
                match settings() {
                    Some(Err(e)) => rsx! { p { class: "muted", "error: {e}" } },
                    _ => rsx! { p { class: "muted", "loading…" } },
                }
            }
        };
    };

    rsx! {
        div { class: "page",
            h1 { "Settings" }
            if !s.is_admin {
                div { class: "card narrow",
                    h3 { "Admin login" }
                    input {
                        class: "search",
                        r#type: "password",
                        placeholder: "admin token",
                        value: "{token}",
                        oninput: move |e| token.set(e.value()),
                        onkeydown: move |e| {
                            if e.key() == Key::Enter {
                                do_login();
                            }
                        },
                    }
                    button { class: "chip", onclick: move |_| do_login(), "Log in" }
                    if let Some(e) = error() {
                        p { class: "muted", "{e}" }
                    }
                }
            } else {
                div { class: "card",
                    h3 { "Monitored repos" }
                    div { class: "check-grid",
                        for r in s.repos.iter() {
                            {
                                let name = r.name.clone();
                                let active = monitored().contains(&name);
                                rsx! {
                                    label { class: "check", key: "{name}",
                                        input {
                                            r#type: "checkbox",
                                            checked: active,
                                            onchange: move |e| {
                                                let mut cur = monitored();
                                                if e.checked() {
                                                    cur.insert(name.clone());
                                                } else {
                                                    cur.remove(&name);
                                                }
                                                monitored.set(cur);
                                            },
                                        }
                                        " {name}"
                                    }
                                }
                            }
                        }
                    }
                    button {
                        class: "chip",
                        onclick: move |_| {
                            let names: Vec<String> = monitored().into_iter().collect();
                            spawn(async move {
                                match api::set_monitored_repos(names).await {
                                    Ok(()) => notice.set(Some("monitored repos saved".into())),
                                    Err(e) => notice.set(Some(format!("save failed: {e}"))),
                                }
                            });
                        },
                        "Save monitored repos"
                    }
                }
                div { class: "card",
                    h3 { "Crates (one per line)" }
                    textarea {
                        class: "crates",
                        rows: 12,
                        value: "{crates_text}",
                        oninput: move |e| crates_text.set(e.value()),
                    }
                    button {
                        class: "chip",
                        onclick: move |_| {
                            let names: Vec<String> = crates_text()
                                .lines()
                                .map(|l| l.trim().to_string())
                                .filter(|l| !l.is_empty())
                                .collect();
                            spawn(async move {
                                match api::set_crates(names).await {
                                    Ok(()) => notice.set(Some("crates saved".into())),
                                    Err(e) => notice.set(Some(format!("save failed: {e}"))),
                                }
                            });
                        },
                        "Save crates"
                    }
                }
                div { class: "card",
                    h3 { "Devin" }
                    p { class: "mono",
                        "configured: {s.devin_configured} · budget today: {s.devin_budget_used}/{s.devin_budget_total}"
                    }
                    input {
                        class: "search",
                        r#type: "number",
                        placeholder: "daily budget",
                        value: "{budget_text}",
                        oninput: move |e| budget_text.set(e.value()),
                    }
                    button {
                        class: "chip",
                        onclick: move |_| {
                            let v = budget_text().parse::<i64>().unwrap_or(s.devin_budget_total);
                            spawn(async move {
                                match api::set_llm_budget(v).await {
                                    Ok(()) => notice.set(Some(format!("budget set to {v}"))),
                                    Err(e) => notice.set(Some(format!("save failed: {e}"))),
                                }
                                *refresh.write() += 1;
                            });
                        },
                        "Save budget"
                    }
                }
                div { class: "card",
                    h3 { "Sync" }
                    div { class: "chips",
                        for kind in ["all", "repos", "prs", "crates", "releases", "snapshots"] {
                            button {
                                class: "chip",
                                onclick: move |_| {
                                    let kind = kind.to_string();
                                    spawn(async move {
                                        match api::sync_now(kind).await {
                                            Ok(m) => notice.set(Some(m)),
                                            Err(e) => notice.set(Some(format!("sync failed: {e}"))),
                                        }
                                        *refresh.write() += 1;
                                    });
                                },
                                "Sync {kind}"
                            }
                        }
                    }
                }
                if let Some(n) = notice() {
                    p { class: "muted", "{n}" }
                }
            }
            div { class: "card",
                h3 { "Sync log" }
                table { class: "data",
                    thead {
                        tr {
                            th { "time" }
                            th { "kind" }
                            th { "repo" }
                            th { "status" }
                            th { "message" }
                        }
                    }
                    tbody {
                        for l in s.sync_log.iter() {
                            tr { class: "row", key: "{l.id}",
                                td { class: "mono muted", "{l.finished_at.clone().unwrap_or_default()}" }
                                td { class: "mono", "{l.kind}" }
                                td { class: "mono", "{l.repo}" }
                                td {
                                    if l.ok {
                                        span { class: "ok-text", "ok" }
                                    } else {
                                        span { class: "fail-text", "fail" }
                                    }
                                }
                                td { class: "muted small", "{l.message}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
