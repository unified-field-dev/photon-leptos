//! Proc macros for Photon Leptos integration.
//!
//! Annotate Leptos server functions for realtime UI — generates client subscription
//! helpers and submits WebSocket route descriptors for inventory discovery.
//!
//! ## [`synced`] attribute reference
//!
//! | Attribute | Required | Default | Description |
//! |-----------|----------|---------|-------------|
//! | `topic` | yes | — | Photon topic name (must match `#[photon::topic]`) |
//! | `ws` | no | `/ws/{fn-with-hyphens}` | WebSocket GET path |
//! | `strategy` | no | `"refetch"` | `"refetch"`, `"replace"`, or `"append"`. `append` requires `Result<Vec<_>, _>` → `synced_resource_append`. `replace` on `Result<T, E>` → `synced_resource_replace_result` (payload is `T`). |
//! | `key` | no | none | Static subscribe key (`?key=` on the WS URL) |
//! | `auth` | no | `"none"` | `"none"` (optional client `?key=`) or `"user"` (host auth at `ws_router`) |
//!
//! ## Generated symbols (for `list_notifications`)
//!
//! | Symbol | Build | Purpose |
//! |--------|-------|---------|
//! | `subscribe_list_notifications(on_event)` | all | Returns trigger `RwSignal`; bumps on WS event |
//! | `use_list_notifications()` | `hydrate` | Full synced [`Resource`](https://docs.rs/leptos) |
//! | `__photon_ws_list_notifications::PATH` | `ssr` | WebSocket path constant |
//! | inventory entry | `ssr` | Auto-discovered by `photon_axum::ws_router` |
//!
//! Requires `photon-leptos` features `hydrate` and/or `ssr` on the app crate.

#![deny(missing_docs)]

mod synced;

use proc_macro::TokenStream;

/// Marks an async server function as a synced Leptos resource backed by Photon events.
///
/// # Example
///
/// ```ignore
/// use photon_leptos::synced;
///
/// #[synced(
///     topic = "notifications.updated",
///     ws = "/ws/notifications",
///     strategy = "refetch",
///     auth = "none",
/// )]
/// pub async fn list_notifications() -> Result<Vec<Notification>, ServerFnError> {
///     Ok(vec![])
/// }
/// ```
///
/// See crate-level docs for the full attribute reference and generated symbols.
#[proc_macro_attribute]
pub fn synced(attr: TokenStream, item: TokenStream) -> TokenStream {
    synced::synced_impl(attr, item)
}
