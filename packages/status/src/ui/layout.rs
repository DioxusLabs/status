use dioxus::prelude::*;

use crate::api;
use crate::Route;

#[derive(Clone, Copy)]
pub struct AdminState(pub Signal<bool>);

#[component]
pub fn Layout() -> Element {
    let mut admin = use_context_provider(|| AdminState(Signal::new(false)));
    use_effect(move || {
        spawn(async move {
            if let Ok(v) = api::is_admin().await {
                admin.0.set(v);
            }
        });
    });
    let path = use_route::<Route>().to_string();
    let is_admin = (admin.0)();

    rsx! {
        header { class: "topnav",
            div { class: "brand", "Dioxus Status" }
            NavItem { to: "/", label: "Overview", path: path.clone() }
            NavItem { to: "/prs", label: "Pull Requests", path: path.clone() }
            NavItem { to: "/issues", label: "Issues", path: path.clone() }
            NavItem { to: "/repos", label: "Repos", path: path.clone() }
            NavItem { to: "/releases", label: "Releases", path: path.clone() }
            NavItem { to: "/health", label: "Health", path: path.clone() }
            NavItem { to: "/bots", label: "Bots", path: path.clone() }
            NavItem { to: "/settings", label: "Settings", path: path.clone() }
            div { class: "spacer" }
            if is_admin {
                span { class: "badge assoc", "admin" }
            }
        }
        main { class: "content", Outlet::<Route> {} }
    }
}

#[component]
fn NavItem(to: &'static str, label: &'static str, path: String) -> Element {
    let active = if to == "/" {
        path == "/"
    } else {
        path.starts_with(to)
    };
    rsx! {
        Link { class: if active { "nav-item active" } else { "nav-item" }, to, "{label}" }
    }
}
