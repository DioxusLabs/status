use std::time::Duration;

use dioxus::{
    core::Task,
    dioxus_core::SpawnIfAsync,
    prelude::{Callback, spawn, use_hook},
    signals::Signal,
};
use futures::{SinkExt, StreamExt, channel::mpsc};

pub async fn sleep(duration: Duration) {
    #[cfg(not(target_family = "wasm"))]
    tokio::time::sleep(duration).await;

    #[cfg(target_family = "wasm")]
    gloo_timers::future::sleep(duration).await;
}

pub struct UseTimeout<Args: 'static> {
    duration: Duration,
    sender: Signal<mpsc::UnboundedSender<Args>>,
}

impl<Args> UseTimeout<Args> {
    pub fn action(&self, args: Args) -> TimeoutHandle {
        let mut sender = (self.sender)();
        let duration = self.duration;
        let handle = spawn(async move {
            sleep(duration).await;
            let _ = sender.send(args).await;
        });
        TimeoutHandle { handle }
    }
}

impl<Args> Clone for UseTimeout<Args> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<Args> Copy for UseTimeout<Args> {}
impl<Args> PartialEq for UseTimeout<Args> {
    fn eq(&self, other: &Self) -> bool {
        self.duration == other.duration && self.sender == other.sender
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeoutHandle {
    handle: Task,
}

impl TimeoutHandle {
    pub fn cancel(self) {
        self.handle.cancel();
    }
}

pub fn use_timeout<Args: 'static, MaybeAsync: SpawnIfAsync<Marker>, Marker>(
    duration: Duration,
    callback: impl FnMut(Args) -> MaybeAsync + 'static,
) -> UseTimeout<Args> {
    use_hook(|| {
        let callback = Callback::new(callback);
        let (sender, mut receiver) = mpsc::unbounded();
        spawn(async move {
            while let Some(args) = receiver.next().await {
                callback.call(args);
            }
        });
        UseTimeout {
            duration,
            sender: Signal::new(sender),
        }
    })
}
