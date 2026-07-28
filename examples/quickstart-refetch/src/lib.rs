#![allow(missing_docs)]
//! Quickstart host: mem Photon + WS refetch (broadcast, auth, key, status).

pub mod app;
#[cfg(feature = "ssr")]
pub mod auth;
#[cfg(feature = "ssr")]
pub mod bind_guard;
#[cfg(feature = "ssr")]
pub mod photon_boot;
#[cfg(feature = "ssr")]
pub mod state;
pub mod synced;

pub use app::{shell, App};

#[cfg(feature = "ssr")]
pub use auth::DemoUserAuth;
#[cfg(feature = "ssr")]
pub use bind_guard::ensure_loopback_bind;
#[cfg(feature = "ssr")]
pub use state::{AppState, CounterStore};

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();
    leptos::mount::hydrate_body(App);
}
