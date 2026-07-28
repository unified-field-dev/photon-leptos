//! Teaching pages: broadcast refetch, auth/key isolation, WS status/close.

use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

use crate::synced::{
    counter_get, counter_get_auth, counter_get_keyed, increment_broadcast, increment_partition,
    DemoPartition,
};

fn sync_cookies(user: Option<&str>, key: Option<&str>) {
    #[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
    {
        use leptos::web_sys;
        use wasm_bindgen::JsCast;
        if let Some(document) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.dyn_into::<web_sys::HtmlDocument>().ok())
        {
            if let Some(user) = user {
                let _ = document.set_cookie(&format!("demo_user={user}; path=/"));
            }
            if let Some(key) = key {
                let _ = document.set_cookie(&format!("demo_key={key}; path=/"));
            }
        }
    }
    let _ = (user, key);
}

fn query_param(name: &str) -> Option<String> {
    let query = use_query_map();
    query.get().get(name).filter(|s| !s.is_empty())
}

#[component]
pub fn HomePage() -> impl IntoView {
    view! { <BroadcastView/> }
}

#[component]
pub fn AuthPage() -> impl IntoView {
    let user = move || query_param("user").unwrap_or_default();
    view! { <AuthView user=user /> }
}

#[component]
pub fn KeyPage() -> impl IntoView {
    let key = move || query_param("key").unwrap_or_default();
    view! { <KeyView key=key /> }
}

#[component]
fn BroadcastView() -> impl IntoView {
    let status_label = RwSignal::new("…".to_string());
    let last_error = RwSignal::new(None::<String>);
    let close_slot: RwSignal<Option<std::sync::Arc<dyn Fn() + Send + Sync>>> = RwSignal::new(None);

    let trigger = RwSignal::new(0u64);
    #[cfg(feature = "hydrate")]
    {
        let handle = photon_leptos::subscribe_ws("/ws/counter", None, move |_| {
            trigger.update(|n| *n += 1);
        });
        Effect::new({
            let status = handle.status;
            move |_| {
                status_label.set(format!("{:?}", status.get()));
            }
        });
        Effect::new({
            let err = handle.last_error;
            move |_| {
                last_error.set(err.get());
            }
        });
        let close = std::sync::Arc::new(move || handle.close());
        close_slot.set(Some(close));
    }

    let counter = Resource::new(move || trigger.get(), move |_| counter_get());

    let on_increment = move |_| {
        leptos::task::spawn_local(async {
            let _ = increment_broadcast().await;
        });
    };
    let on_close = move |_| {
        if let Some(close) = close_slot.get() {
            close();
        }
    };

    view! {
        <h1>"Broadcast refetch"</h1>
        <p>"Mem Photon → WS → Resource refetch. Lab Origin: allow-all (not for production)."</p>
        <div class="row">
            <span>"Counter: "</span>
            <CounterDisplay resource=counter />
            <button type="button" on:click=on_increment>"Increment"</button>
        </div>
        <div class="row">
            <span>"Status: "</span>
            <code>{move || status_label.get()}</code>
            <button type="button" on:click=on_close>"Close WS"</button>
        </div>
        <Show when=move || last_error.get().is_some()>
            <p>"Last error: "{move || last_error.get().unwrap_or_default()}</p>
        </Show>
    }
}

#[component]
fn AuthView(user: impl Fn() -> String + Send + Sync + Clone + 'static) -> impl IntoView {
    let user_now = user();
    if !user_now.is_empty() {
        provide_context(DemoPartition(user_now.clone()));
    }
    sync_cookies((!user_now.is_empty()).then_some(user_now.as_str()), None);

    let trigger = crate::synced::subscribe_counter_get_auth(|| {});
    let counter = Resource::new(move || trigger.get(), move |_| counter_get_auth());

    let on_increment = move |_| {
        let p = user_now.clone();
        leptos::task::spawn_local(async move {
            let _ = increment_partition(p).await;
        });
    };

    view! {
        <h1>"Auth isolation"</h1>
        <p>"User: "<code>{move || user()}</code>" — open alice and bob in two tabs to compare."</p>
        <div class="row">
            <span>"Counter: "</span>
            <CounterDisplay resource=counter />
            <button type="button" on:click=on_increment>"Increment my partition"</button>
        </div>
    }
}

#[component]
fn KeyView(key: impl Fn() -> String + Send + Sync + Clone + 'static) -> impl IntoView {
    let key_now = key();
    if !key_now.is_empty() {
        provide_context(DemoPartition(key_now.clone()));
    }
    sync_cookies(None, (!key_now.is_empty()).then_some(key_now.as_str()));

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

    let on_increment = move |_| {
        let p = key_now.clone();
        leptos::task::spawn_local(async move {
            let _ = increment_partition(p).await;
        });
    };

    view! {
        <h1>"Key isolation"</h1>
        <p>"Key: "<code>{move || key()}</code>" — room-1 vs room-2 stay isolated."</p>
        <div class="row">
            <span>"Counter: "</span>
            <CounterDisplay resource=counter />
            <button type="button" on:click=on_increment>"Increment this key"</button>
        </div>
    }
}

#[component]
fn CounterDisplay(resource: Resource<Result<u64, ServerFnError>>) -> impl IntoView {
    view! {
        <Suspense fallback=move || view! { <span>"…"</span> }>
            {move || match resource.get() {
                Some(Ok(value)) => view! { <strong>{value.to_string()}</strong> }.into_any(),
                Some(Err(err)) => view! { <span>{err.to_string()}</span> }.into_any(),
                None => view! { <span>"…"</span> }.into_any(),
            }}
        </Suspense>
    }
}
