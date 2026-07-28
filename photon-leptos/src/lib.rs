//! # photon-leptos — Leptos integration for Photon events
//!
//! photon-leptos keeps Leptos `Resource`s in sync with Photon events via WebSocket,
//! so UI updates when relevant events are published — without hand-rolling WS wiring.
//!
//! See the [repository README](https://github.com/unified-field-dev/photon-leptos) for a
//! quick-start hero example.
//!
//! ## Architecture
//!
//! ```text
//! Server (any path) --publish--> Photon topic
//!                                      |
//!                                      v
//!                              Axum WS handler
//!                                      |
//!                                      v
//! Browser: subscribe_<fn> trigger --> Resource refetch --> UI
//! ```
//!
//! ```mermaid
//! sequenceDiagram
//!     participant Job as BackgroundJob
//!     participant Photon as PhotonRuntime
//!     participant WS as ws_endpoint
//!     participant Sub as subscribe_helper
//!     participant Res as Resource
//!     participant UI as PageView
//!
//!     Job->>Photon: topic.publish
//!     Photon->>WS: stream event
//!     WS->>Sub: WebSocket envelope
//!     Sub->>Sub: trigger bump
//!     Sub->>Res: refetch synced read fn
//!     Res->>UI: updated view
//! ```
//!
//! Publish can originate from any server path (background job, webhook, mutation handler).
//! Subscribers only need the topic name and a synced read server function.
//!
//! ## Guarantees
//!
//! - **0.1 experimental** — browser WebSocket is an ephemeral invalidation / live-update channel.
//! - **Refetch** — supported; server function remains authoritative (preferred after reconnect).
//! - **Replace** — experimental; payload is `T` or the `Ok` type of `Result<T, E>`.
//! - **Append** — best-effort live tail (buffers during initial load); pair with Refetch for authoritative lists.
//! - **WebSocket endpoint** — server forwards topic streams to browser clients.
//! - **Type-safe topics** — use [`photon::topic`] types for publish and subscribe.
//! - **Declarative API** — [`synced`] macro or `synced_resource` helpers (hydrate).
//! - **Keyed subscriptions** — optional partition filter (e.g. per-user scoping).
//! - **Reconnection** — client WebSocket reconnects on disconnect.
//! - **Lifecycle** — subscriptions clean up when the reactive owner is disposed.
//! - **Observability** — `subscribe_ws` returns `PhotonWsHandle` with status / last error / close (hydrate).
//!
//! ## Core concepts
//!
//! **Synced resource** — a Leptos `Resource` wired to a Photon topic. When an event
//! arrives on the WebSocket, the resource refetches or applies the configured strategy.
//!
//! **WebSocket endpoint** — an Axum GET handler that subscribes to a Photon topic and
//! forwards serialized events to connected clients. Registered automatically via
//! [`photon_axum::ws_router`] when using [`synced`], or manually via [`server::ws::synced_ws_handler`].
//!
//! **Event strategy** — how incoming events update UI state ([`SyncStrategy`]):
//! refetch re-calls the server function; replace writes the WS payload directly
//! (`T`, or `Ok` of `Result<T, E>` via `synced_resource_replace_result`);
//! append is a best-effort live tail via `synced_resource_append`.
//!
//! **Subscription handle** — `subscribe_ws` returns `PhotonWsHandle` with
//! reactive `WsConnectionStatus`, `last_error`, and `close()`.
//! `use_topic_subscription` exposes the same signals on `PhotonSubscription`.
//! Enable the `hydrate` feature for these client APIs.
//!
//! ## Quick flow
//!
//! ### 1. Define a topic
//!
//! Use [`photon::topic`] in shared/server code (the **photon** crate API — not this crate):
//!
//! ```rust,ignore
//! use photon::topic;
//! use serde::{Deserialize, Serialize};
//!
//! #[topic(name = "notifications.updated")]
//! #[derive(Clone, Debug, Serialize, Deserialize)]
//! pub struct NotificationUpdated {
//!     pub user_id: String,
//! }
//! ```
//!
//! ### 2. Annotate a synced read server function
//!
//! Pair `#[server]` with [`synced`]. The macro generates `subscribe_list_notifications` and
//! registers a WS route for inventory discovery:
//!
//! ```rust,ignore
//! use leptos::prelude::*;
//! use photon_leptos::synced;
//!
//! #[server]
//! #[synced(
//!     topic = "notifications.updated",
//!     ws = "/ws/notifications",
//!     strategy = "refetch",
//!     auth = "none",
//! )]
//! pub async fn list_notifications() -> Result<Vec<Notification>, ServerFnError> {
//!     // Load current notifications from your store / DB.
//!     Ok(load_notifications().await?)
//! }
//! ```
//!
//! ### 3. Publish after a mutation
//!
//! Any server path can publish — background job, webhook, or another user's write:
//!
//! ```rust,ignore
//! async fn on_import_job_finished(user_id: String) -> Result<(), Box<dyn std::error::Error>> {
//!     // Persist the new notification first, then notify subscribers.
//!     NotificationUpdated { user_id }.publish().await?;
//!     Ok(())
//! }
//! ```
//!
//! ### 4. Subscribe in the Leptos UI
//!
//! Wire the generated trigger into a `Resource` so the UI refetches on each event:
//!
//! ```rust,ignore
//! use leptos::prelude::*;
//!
//! #[component]
//! pub fn NotificationsPage() -> impl IntoView {
//!     let trigger = subscribe_list_notifications(|| {});
//!     let items = Resource::new(
//!         move || trigger.get(),
//!         move |_| list_notifications(),
//!     );
//!
//!     view! {
//!         <Suspense fallback=move || view! { <p>"Loading…"</p> }>
//!             {move || match items.get() {
//!                 Some(Ok(list)) => view! {
//!                     <ul>
//!                         {list.into_iter().map(|n| view! { <li>{n.title}</li> }).collect_view()}
//!                     </ul>
//!                 }.into_any(),
//!                 Some(Err(err)) => view! { <p>{err.to_string()}</p> }.into_any(),
//!                 None => view! { <p>"Loading…"</p> }.into_any(),
//!             }}
//!         </Suspense>
//!     }
//! }
//! ```
//!
//! ### 5. Mount WS routes at host boot
//!
//! App state must implement [`photon_axum::HasPhoton`]. Call [`photon_axum::ws_router`]
//! (re-exported from [`server`]) so inventory routes like `/ws/notifications` are mounted:
//! production hosts must also implement an Origin allowlist because the default
//! rejects all WebSocket origins.
//!
//! ```rust,ignore
//! use std::sync::Arc;
//!
//! use axum::Router;
//! use photon::Photon;
//! use photon_axum::{HasPhoton, HeadlessWsAuth, ws_router};
//!
//! #[derive(Clone)]
//! struct AppState {
//!     photon: Arc<Photon>,
//! }
//!
//! impl HasPhoton for AppState {
//!     fn photon_arc(&self) -> Arc<Photon> {
//!         Arc::clone(&self.photon)
//!     }
//!
//!     fn allow_ws_origin(&self, origin: Option<&str>) -> bool {
//!         matches!(origin, Some("https://app.example.com"))
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let photon = /* PhotonBuilder::new()...build()? */;
//!     let state = AppState { photon: Arc::new(photon) };
//!
//!     let app = Router::new();
//!     // …leptos_routes / API routes…
//!     let app = ws_router::<AppState, HeadlessWsAuth>(app).with_state(state);
//!
//!     // axum::serve(listener, app).await.unwrap();
//! }
//! ```
//!
//! ## Feature flags
//!
//! | Feature | Enables |
//! |---------|---------|
//! | `hydrate` | [`subscribe_ws`], [`synced_resource`], macro client hooks |
//! | `ssr` | [`server`] re-exports, [`inventory`] for route discovery |
//!
//! Enable both on app crates that compile server and client targets.
//!
//! ## Modules
//!
//! - **client** (`hydrate`) — WebSocket subscription primitives and synced resources
//! - [`server`] (`ssr`) — re-exports from `photon_axum` for Axum boot wiring
//! - **opts** — [`SyncStrategy`] and [`SyncedResourceOpts`]
//! - **error** — [`PhotonLeptosError`]
//!
//! Host integrators should also read [`photon_axum`](https://docs.rs/photon_axum) for
//! `ws_router`, `HasPhoton`, and `PhotonUserExtractor`.

#![warn(missing_docs)]

mod error;
mod opts;
mod ws_url;

pub use error::PhotonLeptosError;
pub use opts::{SyncStrategy, SyncedResourceOpts};
pub use photon_leptos_macros::synced;
pub use ws_url::{ws_url_log_fields, ws_url_with_key};

cfg_if::cfg_if! {
    if #[cfg(feature = "hydrate")] {
        mod client;
        pub use client::{
            subscribe_ws, synced_resource, synced_resource_append, synced_resource_replace_result,
            use_topic_subscription, PhotonSubscription, PhotonWsHandle, WsConnectionStatus,
        };
    }
}

#[cfg(feature = "ssr")]
/// SSR-side WebSocket route registration (re-exports from [`photon_axum`]).
pub mod server;

#[cfg(feature = "ssr")]
pub use quark::inventory;
