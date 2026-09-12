mod api;
mod components;
mod model;
mod ui;

#[cfg(feature = "server")]
mod backend;

use dioxus::prelude::*;
use ui::{
    Bots, Health, Issues, Layout, Overview, PullRequests, Releases, RepoDetail, Repos, Settings,
};

#[derive(Routable, Clone, PartialEq)]
enum Route {
    #[layout(Layout)]
    #[route("/")]
    Overview {},
    #[route("/prs")]
    PullRequests {},
    #[route("/issues")]
    Issues {},
    #[route("/repos")]
    Repos {},
    #[route("/repos/:name")]
    RepoDetail { name: String },
    #[route("/releases")]
    Releases {},
    #[route("/health")]
    Health {},
    #[route("/bots")]
    Bots {},
    #[route("/settings")]
    Settings {},
}

fn main() {
    #[cfg(feature = "server")]
    {
        let _ = dotenvy::dotenv();
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,sqlx=warn")),
            )
            .init();
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(run_server())
            .unwrap();
    }

    #[cfg(not(feature = "server"))]
    dioxus::launch(app);
}

#[cfg(feature = "server")]
async fn run_server() -> anyhow::Result<()> {
    backend::db::init().await?;
    backend::collector::start();

    let addr = backend::env::port()
        .map(|p| std::net::SocketAddr::from(([0, 0, 0, 0], p)))
        .unwrap_or_else(dioxus::cli_config::fullstack_address_or_localhost);

    let router = dioxus::server::axum::Router::new()
        .serve_dioxus_application(dioxus::server::ServeConfig::new(), app);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("listening on {addr}");
    dioxus::server::axum::serve(listener, router).await?;
    Ok(())
}

fn app() -> Element {
    #[cfg(feature = "web")]
    use_effect(|| {
        document::eval("document.documentElement.setAttribute('data-theme', 'dark')");
    });
    rsx! {
        document::Stylesheet { href: asset!("/assets/dx-components-theme.css") }
        document::Stylesheet { href: asset!("/assets/main.css") }
        Router::<Route> {}
    }
}
