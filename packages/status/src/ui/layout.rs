use dioxus::prelude::*;

use crate::Route;

#[component]
pub fn Layout() -> Element {
    rsx! {
        div { class: "shell",
            nav { class: "sidebar",
                div { class: "brand", "Dioxus Status" }
                NavItem { to: "/", label: "Overview" }
                NavItem { to: "/prs", label: "Pull Requests" }
                NavItem { to: "/issues", label: "Issues" }
                NavItem { to: "/repos", label: "Repos" }
                NavItem { to: "/releases", label: "Releases" }
                NavItem { to: "/health", label: "Health" }
                NavItem { to: "/bots", label: "Bots" }
                NavItem { to: "/settings", label: "Settings" }
            }
            main { class: "content", Outlet::<Route> {} }
        }
    }
}

#[component]
fn NavItem(to: &'static str, label: &'static str) -> Element {
    rsx! {
        Link { class: "nav-item", to, "{label}" }
    }
}
