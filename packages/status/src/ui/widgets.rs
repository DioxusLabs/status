use dioxus::prelude::*;

use crate::model::PrSummary;

#[component]
pub fn Sparkline(
    data: Vec<(String, i64)>,
    #[props(default = 120.0)] width: f64,
    #[props(default = 32.0)] height: f64,
) -> Element {
    if data.len() < 2 {
        return rsx! { div { class: "sparkline empty" } };
    }
    let max = data.iter().map(|(_, v)| *v).max().unwrap_or(1).max(1) as f64;
    let min = data.iter().map(|(_, v)| *v).min().unwrap_or(0) as f64;
    let span = (max - min).max(1.0);
    let step = width / (data.len() - 1) as f64;
    let points: String = data
        .iter()
        .enumerate()
        .map(|(i, (_, v))| {
            let x = i as f64 * step;
            let y = height - ((*v as f64 - min) / span) * (height - 2.0) - 1.0;
            format!("{x:.1},{y:.1}")
        })
        .collect::<Vec<_>>()
        .join(" ");
    rsx! {
        svg {
            class: "sparkline",
            width: "{width}",
            height: "{height}",
            view_box: "0 0 {width} {height}",
            polyline {
                points: "{points}",
                fill: "none",
                stroke: "var(--accent, #7aa2f7)",
                "stroke-width": "1.5",
            }
        }
    }
}

#[component]
pub fn LineChart(
    data: Vec<(String, i64)>,
    #[props(default = 640.0)] width: f64,
    #[props(default = 180.0)] height: f64,
) -> Element {
    if data.is_empty() {
        return rsx! { div { class: "chart empty", "no data" } };
    }
    let pad_l = 48.0;
    let pad_b = 20.0;
    let w = width - pad_l - 8.0;
    let h = height - pad_b - 8.0;
    let max = data.iter().map(|(_, v)| *v).max().unwrap_or(1).max(1) as f64;
    let step = if data.len() > 1 {
        w / (data.len() - 1) as f64
    } else {
        0.0
    };
    let points: String = data
        .iter()
        .enumerate()
        .map(|(i, (_, v))| {
            let x = pad_l + i as f64 * step;
            let y = 4.0 + h - (*v as f64 / max) * h;
            format!("{x:.1},{y:.1}")
        })
        .collect::<Vec<_>>()
        .join(" ");
    let ticks: Vec<f64> = (0..=4).map(|i| max * i as f64 / 4.0).collect();
    let first = data.first().map(|(d, _)| d.clone()).unwrap_or_default();
    let last = data.last().map(|(d, _)| d.clone()).unwrap_or_default();
    rsx! {
        svg {
            class: "chart",
            view_box: "0 0 {width} {height}",
            style: "width: 100%; max-width: {width}px; height: auto; display: block;",
            for t in ticks.iter() {
                line {
                    x1: "{pad_l}",
                    y1: "{4.0 + h - (t / max) * h}",
                    x2: "{width - 8.0}",
                    y2: "{4.0 + h - (t / max) * h}",
                    stroke: "var(--border, #333)",
                    "stroke-width": "0.5",
                }
                text {
                    x: "{pad_l - 6.0}",
                    y: "{4.0 + h - (t / max) * h + 3.0}",
                    "text-anchor": "end",
                    class: "chart-label",
                    "{format_compact(*t as i64)}"
                }
            }
            polyline {
                points: "{points}",
                fill: "none",
                stroke: "var(--accent, #7aa2f7)",
                "stroke-width": "1.5",
            }
            text { x: "{pad_l}", y: "{height - 4.0}", class: "chart-label", "{first}" }
            text {
                x: "{width - 8.0}",
                y: "{height - 4.0}",
                "text-anchor": "end",
                class: "chart-label",
                "{last}"
            }
        }
    }
}

pub fn format_compact(v: i64) -> String {
    if v >= 1_000_000 {
        format!("{:.1}M", v as f64 / 1_000_000.0)
    } else if v >= 1_000 {
        format!("{:.1}k", v as f64 / 1_000.0)
    } else {
        format!("{v}")
    }
}

#[component]
pub fn ScoreBadge(score: i64) -> Element {
    let class = if score >= 70 {
        "score good"
    } else if score >= 40 {
        "score mid"
    } else {
        "score bad"
    };
    rsx! { span { class: "{class}", "{score}" } }
}

#[component]
pub fn CiDot(state: String) -> Element {
    let class = match state.as_str() {
        "success" => "ci ok",
        "failure" => "ci fail",
        "pending" => "ci pending",
        _ => "ci none",
    };
    rsx! { span { class: "{class}", title: "{state}" } }
}

#[component]
pub fn PrSummaryList(title: &'static str, items: Vec<PrSummary>, empty: &'static str) -> Element {
    rsx! {
        div { class: "card",
            h3 { "{title}" }
            if items.is_empty() {
                p { class: "muted", "{empty}" }
            }
            for p in items {
                a { class: "pr-row", href: "{p.url}", target: "_blank",
                    span { class: "pr-repo", "{p.repo}#{p.number}" }
                    span { class: "pr-title", "{p.title}" }
                    span { class: "pr-author muted", "{p.author}" }
                    span { class: "pr-age muted", "{p.age_days}d" }
                    ScoreBadge { score: p.score }
                    CiDot { state: p.ci_state }
                }
            }
        }
    }
}

pub fn assoc_label(a: &str) -> &'static str {
    match a {
        "MEMBER" | "OWNER" => "maintainer",
        "CONTRIBUTOR" => "contributor",
        "FIRST_TIME_CONTRIBUTOR" | "FIRST_TIMER" => "first-timer",
        "BOT" => "bot",
        _ => "",
    }
}
