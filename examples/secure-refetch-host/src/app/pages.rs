//! Sign-in + auth-scoped refetch UI.

use leptos::prelude::*;

use crate::synced::{secure_counter_get, secure_increment, sign_in, SessionUser};

fn set_session_cookie(cookie: &str) {
    #[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
    {
        use leptos::web_sys;
        use wasm_bindgen::JsCast;
        if let Some(document) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.dyn_into::<web_sys::HtmlDocument>().ok())
        {
            let _ = document.set_cookie(cookie);
        }
    }
    let _ = cookie;
}

#[component]
pub fn HomePage() -> impl IntoView {
    let user = RwSignal::new(String::new());
    let signed_in = RwSignal::new(false);
    let draft = RwSignal::new("alice".to_string());

    let on_sign_in = move |_| {
        let name = draft.get();
        leptos::task::spawn_local(async move {
            match sign_in(name.clone()).await {
                Ok(cookie) => {
                    set_session_cookie(&cookie);
                    user.set(name);
                    signed_in.set(true);
                }
                Err(e) => tracing::warn!(error = %e, "sign_in failed"),
            }
        });
    };

    view! {
        <h1>"Secure refetch host"</h1>
        <p>
            "Origin allowlist + session cookie → "
            <code>"PhotonUserExtractor"</code>
            ". See "
            <code>"HasPhoton::allow_ws_origin"</code>
            " in "
            <code>"state.rs"</code>
            "."
        </p>
        <div class="row">
            <input
                type="text"
                prop:value=move || draft.get()
                on:input=move |ev| draft.set(event_target_value(&ev))
                placeholder="user id"
            />
            <button type="button" on:click=on_sign_in>"Sign in"</button>
            <Show when=move || signed_in.get()>
                <span>"Signed in as "<code>{move || user.get()}</code></span>
            </Show>
        </div>
        <Show
            when=move || signed_in.get()
            fallback=|| view! { <p>"Sign in to open the auth-scoped WebSocket."</p> }
        >
            <SecureCounterPane user=user />
        </Show>
    }
}

#[component]
fn SecureCounterPane(user: RwSignal<String>) -> impl IntoView {
    let u = user.get_untracked();
    provide_context(SessionUser(u.clone()));
    sync_session_cookie_for_refetch(&u);

    let trigger = crate::synced::subscribe_secure_counter_get(|| {});
    let counter = Resource::new(move || trigger.get(), move |_| secure_counter_get());

    let on_increment = move |_| {
        leptos::task::spawn_local(async {
            let _ = secure_increment().await;
        });
    };

    view! {
        <div class="row">
            <span>"Counter: "</span>
            <Suspense fallback=move || view! { <span>"…"</span> }>
                {move || match counter.get() {
                    Some(Ok(v)) => view! { <strong>{v.to_string()}</strong> }.into_any(),
                    Some(Err(e)) => view! { <span>{e.to_string()}</span> }.into_any(),
                    None => view! { <span>"…"</span> }.into_any(),
                }}
            </Suspense>
            <button type="button" on:click=on_increment>"Increment"</button>
        </div>
    }
}

fn sync_session_cookie_for_refetch(user: &str) {
    set_session_cookie(&format!(
        "demo_session={user}; Path=/; SameSite=Lax; Max-Age=86400"
    ));
}
