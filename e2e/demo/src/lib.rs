#![allow(missing_docs)]
pub mod app;
#[cfg(feature = "ssr")]
pub mod auth;
#[cfg(feature = "ssr")]
pub mod bind_guard;
pub mod counter;
#[cfg(feature = "ssr")]
pub mod photon_boot;
#[cfg(feature = "ssr")]
pub mod state;

pub use app::{shell, App};

#[cfg(feature = "ssr")]
pub use auth::E2eUserAuth;
#[cfg(feature = "ssr")]
pub use bind_guard::{ensure_demo_bind_allowed, DEMO_ALLOW_INSECURE_ENV};
#[cfg(feature = "ssr")]
pub use state::{AppState, CounterStore};

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use app::App;
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();
    leptos::mount::hydrate_body(App);
}
