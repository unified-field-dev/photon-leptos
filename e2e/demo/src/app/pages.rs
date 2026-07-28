//! Leptos pages for the counter E2E demo.

use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

use crate::counter::{
    counter_get, counter_get_auth_key, counter_get_auth_user, counter_get_keyed, E2ePartition,
};

/// Mirror query params into cookies for client-side server-fn refetches.
fn sync_cookies(namespace: &str, user: Option<&str>, key: Option<&str>) {
    #[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
    {
        use leptos::web_sys;
        use wasm_bindgen::JsCast;
        if let Some(document) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.dyn_into::<web_sys::HtmlDocument>().ok())
        {
            let _ = document.set_cookie(&format!("e2e_ns={namespace}; path=/"));
            if let Some(user) = user {
                let _ = document.set_cookie(&format!("e2e_user={user}; path=/"));
            }
            if let Some(key) = key {
                let _ = document.set_cookie(&format!("e2e_key={key}; path=/"));
            }
        }
    }
    let _ = (namespace, user, key);
}

fn query_param(name: &str) -> Option<String> {
    let query = use_query_map();
    query.get().get(name).filter(|s| !s.is_empty())
}

#[component]
pub fn CounterPage() -> impl IntoView {
    let query = use_query_map();
    let namespace = move || {
        query
            .get()
            .get("ns")
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "default".to_string())
    };
    let use_ws = move || query.get().get("mode").as_deref() != Some("no-ws");

    view! {
        <Show
            when=use_ws
            fallback=move || view! { <NoWsCounterView namespace=namespace /> }
        >
            <SyncedCounterView namespace=namespace />
        </Show>
    }
}

#[component]
pub fn AuthOnlyPage() -> impl IntoView {
    let namespace = move || query_param("ns").unwrap_or_else(|| "default".into());
    let user = move || query_param("user").unwrap_or_default();

    view! {
        <AuthOnlyView namespace=namespace user=user />
    }
}

#[component]
pub fn KeyOnlyPage() -> impl IntoView {
    let namespace = move || query_param("ns").unwrap_or_else(|| "default".into());
    let key = move || query_param("key").unwrap_or_default();

    view! {
        <KeyOnlyView namespace=namespace key=key />
    }
}

#[component]
pub fn AuthKeyPage() -> impl IntoView {
    let namespace = move || query_param("ns").unwrap_or_else(|| "default".into());
    let user = move || query_param("user").unwrap_or_default();
    let key = move || query_param("key").unwrap_or_default();

    view! {
        <AuthKeyView namespace=namespace user=user key=key />
    }
}

#[component]
fn SyncedCounterView(
    namespace: impl Fn() -> String + Send + Sync + Clone + 'static,
) -> impl IntoView {
    provide_context(namespace());
    sync_cookies(&namespace(), None, None);

    let ws_status = RwSignal::new("connected".to_string());
    let trigger = crate::counter::subscribe_counter_get(|| {});
    let counter = Resource::new(move || trigger.get(), move |_| counter_get());

    view! {
        <span data-testid="ws-status">{move || ws_status.get()}</span>
        <CounterDisplay resource=counter />
    }
}

#[component]
fn NoWsCounterView(
    namespace: impl Fn() -> String + Send + Sync + Clone + 'static,
) -> impl IntoView {
    provide_context(namespace());
    sync_cookies(&namespace(), None, None);

    let counter = Resource::new(|| (), move |_| counter_get());

    view! {
        <span data-testid="ws-status">"disabled"</span>
        <CounterDisplay resource=counter />
    }
}

#[component]
fn AuthOnlyView(
    namespace: impl Fn() -> String + Send + Sync + Clone + 'static,
    user: impl Fn() -> String + Send + Sync + Clone + 'static,
) -> impl IntoView {
    let user_now = user();
    provide_context(namespace());
    if !user_now.is_empty() {
        provide_context(E2ePartition(user_now.clone()));
    }
    sync_cookies(
        &namespace(),
        (!user_now.is_empty()).then_some(user_now.as_str()),
        None,
    );

    let trigger = crate::counter::subscribe_counter_get_auth_user(|| {});
    let counter = Resource::new(move || trigger.get(), move |_| counter_get_auth_user());

    view! {
        <span data-testid="page-mode">"auth-only"</span>
        <span data-testid="user-id">{move || user()}</span>
        <CounterDisplay resource=counter />
    }
}

#[component]
fn KeyOnlyView(
    namespace: impl Fn() -> String + Send + Sync + Clone + 'static,
    key: impl Fn() -> String + Send + Sync + Clone + 'static,
) -> impl IntoView {
    let key_now = key();
    provide_context(namespace());
    if !key_now.is_empty() {
        provide_context(E2ePartition(key_now.clone()));
    }
    sync_cookies(
        &namespace(),
        None,
        (!key_now.is_empty()).then_some(key_now.as_str()),
    );

    let trigger = RwSignal::new(0u64);
    #[cfg(feature = "hydrate")]
    {
        let _ws = photon_leptos::subscribe_ws(
            "/ws/counter-keyed",
            Some(key_now.as_str()).filter(|k| !k.is_empty()),
            move |_| {
                trigger.update(|n| *n += 1);
            },
        );
    }
    let counter = Resource::new(move || trigger.get(), move |_| counter_get_keyed());

    view! {
        <span data-testid="page-mode">"key-only"</span>
        <span data-testid="key-id">{move || key()}</span>
        <CounterDisplay resource=counter />
    }
}

#[component]
fn AuthKeyView(
    namespace: impl Fn() -> String + Send + Sync + Clone + 'static,
    user: impl Fn() -> String + Send + Sync + Clone + 'static,
    key: impl Fn() -> String + Send + Sync + Clone + 'static,
) -> impl IntoView {
    let user_now = user();
    let key_now = key();
    provide_context(namespace());
    if !user_now.is_empty() {
        provide_context(E2ePartition(user_now.clone()));
    }
    sync_cookies(
        &namespace(),
        (!user_now.is_empty()).then_some(user_now.as_str()),
        (!key_now.is_empty()).then_some(key_now.as_str()),
    );

    let trigger = RwSignal::new(0u64);
    #[cfg(feature = "hydrate")]
    {
        let _ws = photon_leptos::subscribe_ws(
            "/ws/counter-auth-key",
            Some(key_now.as_str()).filter(|k| !k.is_empty()),
            move |_| {
                trigger.update(|n| *n += 1);
            },
        );
    }
    let counter = Resource::new(move || trigger.get(), move |_| counter_get_auth_key());

    view! {
        <span data-testid="page-mode">"auth-key"</span>
        <span data-testid="user-id">{move || user()}</span>
        <span data-testid="key-id">{move || key()}</span>
        <CounterDisplay resource=counter />
    }
}

#[component]
fn CounterDisplay(resource: Resource<Result<u64, ServerFnError>>) -> impl IntoView {
    view! {
        <Suspense fallback=move || view! {
            <span data-testid="counter-loading">"…"</span>
        }>
            {move || match resource.get() {
                Some(Ok(value)) => view! {
                    <span data-testid="counter-value">{value.to_string()}</span>
                }.into_any(),
                Some(Err(err)) => view! {
                    <span data-testid="counter-error">{err.to_string()}</span>
                }.into_any(),
                None => view! {
                    <span data-testid="counter-loading">"…"</span>
                }.into_any(),
            }}
        </Suspense>
    }
}
