mod bots;
pub mod cache;
mod health;
mod issues;
mod layout;
mod overview;
mod prs;
mod releases;
mod repo_detail;
mod repos;
mod settings;
pub mod storage;
mod widgets;

pub use bots::Bots;
pub use health::Health;
pub use issues::Issues;
pub use layout::Layout;
pub use overview::Overview;
pub use prs::PullRequests;
pub use releases::{ReleaseDetailPage, Releases};
pub use repo_detail::RepoDetail;
pub use repos::Repos;
pub use settings::Settings;

pub async fn sleep_ms(ms: u64) {
    let dur = std::time::Duration::from_millis(ms);
    #[cfg(feature = "web")]
    gloo_timers::future::sleep(dur).await;
    #[cfg(feature = "server")]
    tokio::time::sleep(dur).await;
}
