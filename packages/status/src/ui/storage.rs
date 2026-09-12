use dioxus::prelude::*;
use serde::{de::DeserializeOwned, Serialize};

/// Persist a signal to `localStorage[key]` as JSON (web only; a no-op on the
/// server). Loads once on mount, then writes back on every change.
pub fn use_persisted_signal<T>(key: &'static str, sig: Signal<T>)
where
    T: Serialize + DeserializeOwned + PartialEq + Clone + 'static,
{
    use_persisted_signal_checked(key, sig, Some);
}

/// Like `use_persisted_signal`, but the loaded value passes through `accept`,
/// which may sanitize it or reject it entirely (by returning None).
pub fn use_persisted_signal_checked<T, F>(key: &'static str, mut sig: Signal<T>, accept: F)
where
    T: Serialize + DeserializeOwned + PartialEq + Clone + 'static,
    F: Fn(T) -> Option<T> + 'static,
{
    #[cfg(feature = "web")]
    {
        let mut loaded = use_signal(|| false);
        use_hook(move || {
            spawn(async move {
                if let Ok(v) =
                    document::eval(&format!("return localStorage.getItem('{key}') ?? ''"))
                        .await
                        .map_err(|e| e.to_string())
                        .map(|v| v.as_str().unwrap_or_default().to_string())
                {
                    if let Some(saved) = (!v.is_empty())
                        .then(|| serde_json::from_str::<T>(&v).ok())
                        .flatten()
                        .and_then(&accept)
                    {
                        sig.set(saved);
                    }
                }
                loaded.set(true);
            });
        });
        use_effect(move || {
            let v = sig();
            if !loaded() {
                return;
            }
            document::eval(&format!(
                "localStorage.setItem('{key}', '{}')",
                serde_json::to_string(&v).unwrap_or_default()
            ));
        });
    }
    #[cfg(not(feature = "web"))]
    {
        let _ = (key, &mut sig, &accept);
    }
}

/// Debounced copy of `source`: updates `ms` after the last source change.
/// Returns a writable signal so callers can still commit a value immediately
/// (e.g. restoring state from the URL on first load).
pub fn use_debounced(source: ReadSignal<String>, ms: u64) -> Signal<String> {
    let mut out = use_signal(|| source.peek().clone());
    let pending = use_hook(|| std::rc::Rc::new(std::cell::Cell::new(0u32)));
    use_effect(move || {
        let v = source();
        let rev = pending.get() + 1;
        pending.set(rev);
        let pending = pending.clone();
        spawn(async move {
            super::sleep_ms(ms).await;
            if pending.get() == rev && *out.peek() != v {
                out.set(v);
            }
        });
    });
    out
}
