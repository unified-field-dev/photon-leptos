//! Configuration types for synced Leptos resources.
//!
//! [`SyncStrategy`] and [`SyncedResourceOpts`] control how WebSocket events update
//! UI state when using [`crate::synced_resource`] or macro-generated hooks.

#![deny(missing_docs)]

/// How the resource responds to incoming Photon events.
///
/// | Variant | When to use |
/// |---------|-------------|
/// | [`Refetch`](Self::Refetch) | Server owns query logic (lists, joins, auth-scoped reads) |
/// | [`Replace`](Self::Replace) | WS payload is the new value — plain `T`, or the `Ok` type when the server fn returns `Result<T, E>` |
/// | [`Append`](Self::Append) | Best-effort live tail for lists — **only** via `synced_resource_append` (hydrate; not `synced_resource`) |
///
/// `synced_resource` (hydrate) accepts [`Refetch`](Self::Refetch) and [`Replace`](Self::Replace)
/// only; passing [`Append`](Self::Append) panics. For `Result<T, E>` Replace, the
/// `#[synced]` macro calls `synced_resource_replace_result` so events
/// deserialize as `T` and set `Ok(T)`.
///
/// Append is a best-effort live tail: buffers during the initial snapshot; prefer
/// [`Refetch`](Self::Refetch) after reconnect when the full list must match the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SyncStrategy {
    /// Re-call the server function to fetch fresh data.
    #[default]
    Refetch,

    /// Append the event payload to a list (for `Result<Vec<U>, E>`).
    ///
    /// Best-effort: events during the initial snapshot load are buffered; reconnect
    /// may miss or duplicate items. See `synced_resource_append` (hydrate feature).
    Append,

    /// Replace resource data with the event payload.
    ///
    /// Payload type is `T` for a plain return type, or the `Ok` type when the
    /// fetcher returns `Result<T, E>` (via `synced_resource_replace_result`, hydrate).
    Replace,
}

impl std::str::FromStr for SyncStrategy {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "refetch" => Ok(SyncStrategy::Refetch),
            "append" => Ok(SyncStrategy::Append),
            "replace" => Ok(SyncStrategy::Replace),
            _ => Err(()),
        }
    }
}

/// Configuration for a synced resource hook (`synced_resource` / macro-generated).
#[derive(Debug, Clone)]
pub struct SyncedResourceOpts {
    /// Photon topic name (e.g. `"user.notifications"`).
    pub topic: String,

    /// WebSocket endpoint path (e.g. `"/ws/notifications"`).
    pub ws_path: String,

    /// How to apply incoming events.
    pub strategy: SyncStrategy,

    /// Optional Photon subscribe key, sent as `?key=` on the WebSocket URL.
    ///
    /// The server enforces auth + key policy (see `photon_axum::resolve_subscribe_key`):
    /// - `auth = "none"` + key → subscribe with that key
    /// - `auth = "user"` without key → subscribe with the session user key
    /// - `auth = "user"` + key → key must match the session user key
    ///
    /// **`None`:** when `auth = "none"`, omit `?key=` for broadcast, or set a
    /// client-selected key for optional partition scoping. Prefer `auth = "user"`
    /// for authenticated per-user topics.
    pub key_filter: Option<String>,
}
