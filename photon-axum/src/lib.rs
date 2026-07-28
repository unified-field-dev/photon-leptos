//! # photon-axum — Axum WebSocket registration for Photon browser clients
//!
//! Bridges Photon topic streams to browser WebSockets. Routes annotated with
//! [`photon_leptos::synced`](https://docs.rs/photon_leptos_macros) submit [`WsRouteDescriptor`]
//! entries via inventory; [`ws_router`] discovers them at boot and mounts GET handlers.
//!
//! ## Boot checklist
//!
//! 1. App state implements [`HasPhoton`] with `Arc<photon::Photon>`.
//! 2. App state overrides [`HasPhoton::allow_ws_origin`] with a production Origin
//!    allowlist. The default rejects every origin.
//! 3. Binary links crates that use `#[photon_leptos::synced]` (inventory submit).
//! 4. Call [`ws_router`]::<`S`, `Auth`>(app) before serving.
//!
//! ```rust,ignore
//! use photon_axum::{HeadlessWsAuth, ws_router};
//!
//! let app = ws_router::<AppState, HeadlessWsAuth>(router);
//! ```
//!
//! For client hooks and synced resources, use [`photon_leptos`](https://docs.rs/photon_leptos).
//! Typed topic streams can also be bridged manually via `photon::Photon::subscribe` and
//! [`synced_ws_handler`].
//!
//! ## Modules
//!
//! - [`axum_ws`] — auth traits, descriptors, route registration, WS handler

#![cfg(feature = "runtime")]
#![deny(missing_docs)]

pub mod axum_ws;

pub use axum_ws::{
    apply_ws_routes, origin_from_headers, reject_origin, resolve_subscribe_key, synced_ws_handler,
    FanoutConfigError, HasPhoton, HeadlessWsAuth, KeyResolveError, PhotonUserExtractor,
    SyncedWsConfig, WsAuthMode, WsBroadcastHub, WsFanoutMode, WsRouteDescriptor,
};

use axum::Router;

/// Apply all inventory-discovered Photon WebSocket routes to `app`.
///
/// `Auth` is the host's session extractor for routes registered with `auth = "user"`.
/// Use [`HeadlessWsAuth`] for demos and headless servers.
pub fn ws_router<S, Auth>(app: Router<S>) -> Router<S>
where
    S: axum_ws::HasPhoton + Clone + Send + Sync + 'static,
    Auth: axum_ws::PhotonUserExtractor + axum::extract::FromRequestParts<S> + Send + 'static,
    <Auth as axum::extract::FromRequestParts<S>>::Rejection: axum::response::IntoResponse + Send,
{
    axum_ws::apply_ws_routes::<S, Auth>(app)
}
