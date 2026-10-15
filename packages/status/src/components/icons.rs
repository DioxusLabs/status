use dioxus::prelude::*;

#[derive(Clone, PartialEq, Props)]
pub struct IconProps {
    #[props(default)]
    pub size: Option<String>,
    #[props(default)]
    pub stroke: Option<String>,
    #[props(default)]
    pub class: Option<String>,
}

fn svg(props: &IconProps, path: &'static str) -> Element {
    let size = props.size.clone().unwrap_or_else(|| "1rem".into());
    let stroke = props
        .stroke
        .clone()
        .unwrap_or_else(|| "currentColor".into());
    let class = props.class.clone().unwrap_or_default();
    rsx! {
        svg {
            class: "{class}",
            width: "{size}",
            height: "{size}",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "{stroke}",
            "stroke-width": "2",
            "stroke-linecap": "round",
            "stroke-linejoin": "round",
            dangerous_inner_html: "{path}",
        }
    }
}

#[component]
pub fn Check(props: IconProps) -> Element {
    svg(&props, "<polyline points='20 6 9 17 4 12'/>")
}

#[component]
pub fn ChevronDown(props: IconProps) -> Element {
    svg(&props, "<polyline points='6 9 12 15 18 9'/>")
}

#[component]
pub fn ChevronsUpDown(props: IconProps) -> Element {
    svg(
        &props,
        "<polyline points='7 15 12 20 17 15'/><polyline points='7 9 12 4 17 9'/>",
    )
}

#[component]
pub fn BadgeCheck(props: IconProps) -> Element {
    svg(
        &props,
        "<path d='M3.85 8.62a4 4 0 0 1 4.78-4.77 4 4 0 0 1 6.74 0 4 4 0 0 1 4.78 4.78 4 4 0 0 1 0 6.74 4 4 0 0 1-4.77 4.78 4 4 0 0 1-6.75 0 4 4 0 0 1-4.78-4.77 4 4 0 0 1 0-6.76Z'/><path d='m9 12 2 2 4-4'/>",
    )
}
