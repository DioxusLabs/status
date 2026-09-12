use dioxus::prelude::*;
use dioxus::{fullstack::Loading, CapturedError};
use serde::{de::DeserializeOwned, Serialize};
#[cfg(feature = "web")]
use std::any::Any;

#[cfg(feature = "web")]
thread_local! {
    static CACHE: std::cell::RefCell<std::collections::HashMap<String, Box<dyn Any>>> =
        Default::default();
}

fn cache_get<T: Clone + 'static>(key: &str) -> Option<T> {
    #[cfg(feature = "web")]
    {
        CACHE.with(|cache| {
            cache
                .borrow()
                .get(key)
                .and_then(|value| value.downcast_ref::<T>().cloned())
        })
    }
    #[cfg(not(feature = "web"))]
    {
        let _ = key;
        None
    }
}

fn cache_put<T: Clone + 'static>(key: String, value: &T) {
    #[cfg(feature = "web")]
    CACHE.with(|cache| {
        cache.borrow_mut().insert(key, Box::new(value.clone()));
    });
    #[cfg(not(feature = "web"))]
    {
        let _ = (key, value);
    }
}

pub struct Cached<T: 'static> {
    pub value: ReadSignal<Option<T>>,
    pub loading: ReadSignal<bool>,
    pub error: ReadSignal<Option<String>>,
    restart: Callback<()>,
}

impl<T: 'static> Copy for Cached<T> {}

impl<T: 'static> Clone for Cached<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> Cached<T> {
    pub fn restart(&self) {
        self.restart.call(());
    }
}

/// Load data with a client-side stale-while-revalidate cache. The first load
/// still suspends for SSR and an uncached client visit; subsequent visits show
/// the cached value while the loader refreshes in the background.
#[allow(clippy::result_large_err)]
pub fn use_cached<T, F, E>(
    key: impl Fn() -> String + 'static,
    mut fetch: impl FnMut() -> F + 'static,
) -> Result<Cached<T>, RenderError>
where
    T: Clone + PartialEq + Serialize + DeserializeOwned + 'static,
    F: std::future::Future<Output = Result<T, E>> + 'static,
    E: Into<CapturedError> + 'static,
{
    let key = use_memo(key);
    let mut value = use_signal(|| cache_get::<T>(&key.peek()));
    let mut error = use_signal(|| None::<String>);
    let mut loading = use_signal(|| false);
    let mut restart_tick = use_signal(|| 0u32);

    let loader = use_loader(move || {
        let cache_key = key();
        let _restart = restart_tick();
        let future = fetch();
        async move {
            let result = future.await;
            if let Ok(v) = &result {
                cache_put(cache_key, v);
            }
            result
        }
    });

    let restart = Callback::new(move |_| {
        restart_tick += 1;
    });

    match loader {
        Ok(loader) => {
            let fetched = loader.cloned();
            if value.peek().as_ref() != Some(&fetched) {
                value.set(Some(fetched));
            }
            if error.peek().is_some() {
                error.set(None);
            }
            let busy = loader.loading();
            if *loading.peek() != busy {
                loading.set(busy);
            }
        }
        Err(Loading::Pending(handle)) => {
            if value.peek().is_none() {
                return Err(Loading::Pending(handle).into());
            }
            if !*loading.peek() {
                loading.set(true);
            }
        }
        Err(Loading::Failed(handle)) => {
            let message = handle.error().map(|e| e.to_string());
            if *error.peek() != message {
                error.set(message);
            }
            if *loading.peek() {
                loading.set(false);
            }
            if value.peek().is_none() {
                return Err(Loading::Failed(handle).into());
            }
        }
    }

    Ok(Cached {
        value: value.into(),
        loading: loading.into(),
        error: error.into(),
        restart,
    })
}
