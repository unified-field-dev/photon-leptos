//! Client-side synced resources and WebSocket lifecycle.
//!
//! Helpers for Leptos components that subscribe to Photon topics over WebSocket and
//! keep [`Resource`] data fresh when events arrive.
//!
//! ## Choosing a sync strategy
//!
//! | Strategy | When to use |
//! |---------|-------------|
//! | [`SyncStrategy::Refetch`](crate::SyncStrategy::Refetch) | Server owns query logic (lists, joins, auth-scoped reads) |
//! | [`SyncStrategy::Replace`](crate::SyncStrategy::Replace) | WS payload is the new value (`T`, or `Ok` of `Result<T, E>`) |
//! | [`SyncStrategy::Append`](crate::SyncStrategy::Append) | Best-effort live tail — use [`synced_resource_append`] |
//!
//! ## API tiers
//!
//! | Tier | API | Use when |
//! |------|-----|----------|
//! | 2 (macro) | `use_<fn>()` from `#[photon_leptos::synced]` | Single resource, convention-based |
//! | 1 | [`use_topic_subscription`] → [`PhotonSubscription`] | Multiple resources/effects on one WS path (+ status) |
//! | 0 | [`subscribe_ws`] → [`PhotonWsHandle`] | Custom callback + status / last_error / close |
//!
//! ### Helper example (without macro)
//!
//! ```rust,ignore
//! use photon_leptos::{synced_resource, SyncStrategy, SyncedResourceOpts};
//!
//! pub fn use_notifications() -> Resource<Result<Vec<Notification>, ServerFnError>> {
//!     synced_resource(
//!         list_notifications,
//!         SyncedResourceOpts {
//!             topic: "notifications.updated".into(),
//!             ws_path: "/ws/notifications".into(),
//!             strategy: SyncStrategy::Refetch,
//!             key_filter: None,
//!         },
//!     )
//! }
//! ```
//!
//! On SSR-only builds, WebSocket calls compile out; triggers stay at 0 and initial
//! values come from server-rendered [`Resource`] fetches.
//!
//! ## WebSocket client contract
//!
//! [`subscribe_ws`] documents the leptos-use feature-unification hazard and the
//! `message.get()` + `Effect` pattern required for connections to open. Do not
//! refactor that helper without running browser E2E (see repository `e2e/README.md`).
//!
//! Uses `leptos_use::use_websocket` for connection management (reconnect,
//! cleanup, protocol resolution).

use crate::{ws_url_log_fields, ws_url_with_key, SyncedResourceOpts};
use codee::string::FromToStringCodec;
use leptos::prelude::*;
use leptos_use::core::ConnectionReadyState;
use leptos_use::{use_websocket_with_options, UseWebSocketOptions};
use std::future::Future;
use std::sync::Arc;

/// Photon Event shape (minimal for payload extraction).
#[derive(serde::Deserialize)]
struct PhotonEvent {
    #[serde(rename = "payload_json")]
    payload_json: serde_json::Value,
}

/// Reactive WebSocket connection status for [`subscribe_ws`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WsConnectionStatus {
    /// Handshake in progress.
    Connecting,
    /// Socket is open.
    Open,
    /// Close handshake in progress.
    Closing,
    /// Socket is closed (may reconnect).
    Closed,
}

impl From<ConnectionReadyState> for WsConnectionStatus {
    fn from(value: ConnectionReadyState) -> Self {
        match value {
            ConnectionReadyState::Connecting => Self::Connecting,
            ConnectionReadyState::Open => Self::Open,
            ConnectionReadyState::Closing => Self::Closing,
            ConnectionReadyState::Closed => Self::Closed,
        }
    }
}

/// Handle returned by [`subscribe_ws`] with status, last error, and close.
pub struct PhotonWsHandle {
    /// Mapped from leptos-use `ready_state`.
    pub status: Signal<WsConnectionStatus>,
    /// Last connection/decode error message (if any).
    pub last_error: ReadSignal<Option<String>>,
    close_fn: Arc<dyn Fn() + Send + Sync>,
}

impl PhotonWsHandle {
    /// Close the WebSocket (also runs on reactive owner cleanup).
    pub fn close(&self) {
        (self.close_fn)();
    }
}

// ---------------------------------------------------------------------------
// Public API — Tier 1: shared subscription primitive
// ---------------------------------------------------------------------------

/// Reactive handle to a Photon WebSocket topic subscription.
///
/// Multiple resources and effects can depend on the same `trigger` signal
/// so they all react to the same WS event without opening separate
/// connections.
///
/// # Example — two resources on one topic
///
/// ```rust,ignore
/// let sub = use_topic_subscription("/ws/notifications", Some("1234"));
///
/// let count = Resource::new(move || sub.trigger.get(), move |_| get_unread_count());
/// let list  = Resource::new(move || sub.trigger.get(), move |_| get_list());
///
/// // Connection observability:
/// // sub.status.get() → WsConnectionStatus::{Connecting, Open, Closing, Closed}
/// // sub.last_error.get() → Option<String>
///
/// // Force refresh from outside (e.g. after marking a notification read):
/// sub.refetch();
/// ```
#[derive(Clone, Copy)]
pub struct PhotonSubscription {
    /// Bumped on every incoming WS event. Depend on this to refetch.
    pub trigger: RwSignal<u64>,
    /// The latest event payload (useful for Replace-strategy consumers).
    pub latest_event: RwSignal<Option<serde_json::Value>>,
    /// Connection status from the underlying WebSocket.
    pub status: Signal<WsConnectionStatus>,
    /// Last connection/decode error, if any.
    pub last_error: ReadSignal<Option<String>>,
}

impl PhotonSubscription {
    /// Force a trigger bump from outside the WS callback — e.g. after a
    /// local mutation that should refresh all dependent resources.
    pub fn refetch(&self) {
        self.trigger.update(|n| *n += 1);
    }
}

/// Create a shared subscription to a Photon WebSocket endpoint.
///
/// Returns a [`PhotonSubscription`] whose `trigger` signal bumps on every
/// incoming event. Wire any number of `Resource`s, `Effect`s, or
/// `use_paged_infinite_scroll` hooks to this trigger.
///
/// This is the **Tier 1** primitive. For the common one-resource case, use
/// the `use_<fn_name>()` hook generated by `#[photon_leptos::synced]` instead.
pub fn use_topic_subscription(ws_path: &str, key_filter: Option<&str>) -> PhotonSubscription {
    let trigger = RwSignal::new(0u64);
    let latest_event: RwSignal<Option<serde_json::Value>> = RwSignal::new(None);

    let handle = subscribe_ws(ws_path, key_filter, move |payload| {
        latest_event.set(Some(payload));
        trigger.update(|n| *n += 1);
    });

    PhotonSubscription {
        trigger,
        latest_event,
        status: handle.status,
        last_error: handle.last_error,
    }
}

// ---------------------------------------------------------------------------
// Public API — Tier 0: raw callback subscription
// ---------------------------------------------------------------------------

/// Subscribe to a Photon WebSocket and call `on_event` for each incoming
/// message payload.  The connection is automatically managed by
/// `leptos_use::use_websocket` (reconnect, cleanup on owner disposal).
///
/// Returns a [`PhotonWsHandle`] with reactive status, last error, and
/// [`PhotonWsHandle::close`]. Prefer [`use_topic_subscription`] for most use cases.
///
/// # Implementation contract (do not change without E2E verification)
///
/// ## Cargo feature-unification hazard
///
/// `leptos-use` uses `#[cfg(feature = "ssr")]` internally: when `ssr` is
/// active, `use_websocket_with_options` compiles the `open`/`close`/`send`
/// closures as **no-ops** and the WebSocket never connects. Cargo unifies
/// features **per package across the entire target**, so if **any** crate
/// in the dependency graph enables `leptos-use/ssr` unconditionally (even
/// a crate unrelated to WebSockets), every crate's leptos-use gets `ssr`
/// and the WebSocket silently dies on the client. The symptom is:
/// `subscribing to WebSocket` logs in the browser, but no `on_open`,
/// `on_message`, or connection activity at all.
///
/// **Rule:** every crate that depends on `leptos-use` must gate `ssr`
/// behind its own `ssr` feature — never put it in unconditional
/// `[dependencies]` features.
///
/// ## `message.get()` + `Effect` pattern
///
/// The connection lifecycle depends on **destructuring `UseWebSocketReturn`
/// and reading `message.get()` inside an `Effect`**:
///
/// 1. `let UseWebSocketReturn { message, .. }` — the `message` signal is a
///    reactive handle into leptos-use's internal connection state.
/// 2. `Effect::new(move |_| { message.get(); … })` — the `Effect` registers
///    with the current reactive owner and the `.get()` call subscribes to the
///    signal, which keeps leptos-use's internal connection `Effect` tracked
///    and scheduled.
/// 3. Dropping the `UseWebSocketReturn` (e.g. `let _ = use_websocket_with_options(…)`)
///    or replacing `message.get()` with `.on_message()` callbacks alone will
///    cause the WebSocket to never connect.
///
/// **Do not** refactor to `.on_message()` callbacks without the `message.get()`
/// `Effect`, and **do not** bind the return value to `_`.
///
/// `key_filter` is appended as `?key=` (see [`crate::ws_url_with_key`]).
pub fn subscribe_ws(
    ws_path: &str,
    key_filter: Option<&str>,
    on_event: impl Fn(serde_json::Value) + Send + Sync + 'static,
) -> PhotonWsHandle {
    let url = ws_url_with_key(ws_path, key_filter);
    let (log_path, has_key) = ws_url_log_fields(&url);
    tracing::info!(
        ws_path = %log_path,
        has_key,
        "[photon-leptos] subscribing to WebSocket"
    );

    let last_error = RwSignal::new(None::<String>);
    let last_error_for_cb = last_error;

    let socket = use_websocket_with_options::<String, String, FromToStringCodec, _, _>(
        &url,
        UseWebSocketOptions::default()
            .reconnect_limit(leptos_use::ReconnectLimit::Limited(u64::MAX))
            .reconnect_interval(3_000)
            .on_error({
                let log_path = log_path.to_owned();
                move |err| {
                    let msg = format!("{err:?}");
                    tracing::warn!(
                        ws_path = %log_path,
                        has_key,
                        error = %msg,
                        "[photon-leptos] WebSocket error"
                    );
                    last_error_for_cb.set(Some(msg));
                }
            }),
    );

    let message = socket.message;
    let ready_state = socket.ready_state;
    let status = Signal::derive(move || WsConnectionStatus::from(ready_state.get()));
    let close_fn: Arc<dyn Fn() + Send + Sync> = {
        let close = socket.close.clone();
        Arc::new(close)
    };

    // Keep connection control handles alive for the reactive owner lifetime.
    on_cleanup({
        let close = Arc::clone(&close_fn);
        move || {
            close();
        }
    });
    Effect::new({
        let socket = socket.clone();
        move |_| {
            let _ = &socket;
        }
    });

    Effect::new(move |_prev| {
        if let Some(text) = message.get() {
            match serde_json::from_str::<PhotonEvent>(&text) {
                Ok(photon_ev) => {
                    tracing::info!("[photon-leptos] received event on WS, dispatching");
                    on_event(photon_ev.payload_json);
                }
                Err(e) => {
                    let preview: String = text.chars().take(256).collect();
                    let msg = format!("not a Photon event envelope ({e}): {preview:?}");
                    tracing::warn!(message = %msg, "[photon-leptos] WebSocket message");
                    last_error.set(Some(msg));
                }
            }
        }
    });

    PhotonWsHandle {
        status,
        last_error: last_error.read_only(),
        close_fn,
    }
}

/// Creates a Leptos resource that stays in sync with Photon events via WebSocket.
///
/// When an event is received on the WebSocket, the resource is refetched (or the
/// configured strategy is applied). The WebSocket connection is closed when the
/// reactive owner is disposed (component unmount).
///
/// # Arguments
///
/// * `fetcher` - Closure that returns the async fetch (e.g. a server function call)
/// * `opts` - Configuration for topic, WebSocket path, and sync strategy
///
/// Only [`crate::SyncStrategy::Refetch`] and [`crate::SyncStrategy::Replace`] are valid here.
/// For appendable lists, call [`synced_resource_append`] instead (the `#[synced]` macro
/// routes `strategy = "append"` automatically).
///
/// # Panics
///
/// Panics if `opts.strategy` is [`crate::SyncStrategy::Append`]. That is a programmer
/// error — use [`synced_resource_append`].
pub fn synced_resource<F, Fut, T>(fetcher: F, opts: SyncedResourceOpts) -> Resource<T>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = T> + Send + 'static,
    T: serde::Serialize + serde::de::DeserializeOwned + Send + Sync + Clone + PartialEq + 'static,
{
    match opts.strategy {
        crate::SyncStrategy::Refetch => synced_resource_refetch(fetcher, opts),
        crate::SyncStrategy::Replace => synced_resource_replace(fetcher, opts),
        crate::SyncStrategy::Append => {
            panic!(
                "SyncStrategy::Append requires synced_resource_append; do not pass Append to synced_resource"
            );
        }
    }
}

/// Creates a synced resource for appendable lists. When an event arrives, the
/// payload is deserialized as `U` and appended to the current list.
///
/// Returns `Resource<Option<Result<Vec<U>, E>>>` — `None` while loading.
///
/// **Best-effort live tail:** events that arrive while the initial snapshot is
/// still loading are buffered and flushed when the fetch completes `Ok`. Prefer
/// [`SyncStrategy::Refetch`](crate::SyncStrategy::Refetch) after reconnect when the
/// server function must remain authoritative.
pub fn synced_resource_append<F, Fut, U, E>(
    fetcher: F,
    opts: SyncedResourceOpts,
) -> Resource<Option<Result<Vec<U>, E>>>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Vec<U>, E>> + Send + 'static,
    U: serde::Serialize + serde::de::DeserializeOwned + Send + Sync + Clone + PartialEq + 'static,
    E: Send + Sync + Clone + PartialEq + serde::Serialize + serde::de::DeserializeOwned + 'static,
{
    use leptos::task::spawn_local_scoped;

    let data_signal: RwSignal<Option<Result<Vec<U>, E>>> = RwSignal::new(None);
    let pending: RwSignal<Vec<U>> = RwSignal::new(Vec::new());
    let data_clone = data_signal;
    let pending_for_fetch = pending;

    spawn_local_scoped(async move {
        let val = fetcher().await;
        match &val {
            Ok(_) => {
                let buffered = pending_for_fetch.get_untracked();
                pending_for_fetch.set(Vec::new());
                if buffered.is_empty() {
                    data_clone.set(Some(val));
                } else if let Ok(mut list) = val {
                    list.extend(buffered);
                    data_clone.set(Some(Ok(list)));
                }
            }
            Err(_) => {
                pending_for_fetch.set(Vec::new());
                data_clone.set(Some(val));
            }
        }
    });

    let data_clone_for_res = data_signal;
    let resource = Resource::new(
        move || data_signal.get(),
        move |_| std::future::ready(data_clone_for_res.get()),
    );

    let data_for_ws = data_signal;
    let pending_for_ws = pending;
    let key = opts.key_filter.clone();
    let ws_path_for_log = opts.ws_path.clone();
    subscribe_ws(
        &opts.ws_path,
        key.as_deref(),
        move |payload_json| match serde_json::from_value::<U>(payload_json) {
            Ok(item) => match data_for_ws.get_untracked() {
                None => {
                    pending_for_ws.update(|buf| buf.push(item));
                }
                Some(Err(_)) => {
                    tracing::warn!(
                        ws_path = %ws_path_for_log,
                        "[photon-leptos] Append: skipping item (list is in error state)"
                    );
                }
                Some(Ok(_)) => {
                    data_for_ws.update(|opt| {
                        if let Some(Ok(vec)) = opt {
                            vec.push(item);
                        }
                    });
                }
            },
            Err(e) => {
                tracing::warn!(
                    ws_path = %ws_path_for_log,
                    error = %e,
                    "[photon-leptos] Append: payload deserialization failed"
                );
            }
        },
    );

    resource
}

// ---------------------------------------------------------------------------
// Internal
// ---------------------------------------------------------------------------

/// Replace strategy: initial value from the server function (SSR safe),
/// then subsequent updates written directly from the WS payload — no
/// server round-trip.  Falls back to refetch if the payload can't be
/// deserialized into `T`.
fn synced_resource_replace<F, Fut, T>(fetcher: F, opts: SyncedResourceOpts) -> Resource<T>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = T> + Send + 'static,
    T: serde::Serialize + serde::de::DeserializeOwned + Send + Sync + Clone + PartialEq + 'static,
{
    let trigger = RwSignal::new(0u64);
    let direct_value: RwSignal<Option<T>> = RwSignal::new(None);

    let resource = Resource::new(
        move || (trigger.get(), direct_value.get()),
        move |(_, injected)| {
            let fetcher_fut = fetcher();
            async move {
                if let Some(val) = injected {
                    val
                } else {
                    fetcher_fut.await
                }
            }
        },
    );

    tracing::info!(
        topic = %opts.topic,
        ws_path = %opts.ws_path,
        strategy = "Replace",
        "[photon-leptos] synced_resource"
    );

    let ws_path_for_log = opts.ws_path.clone();
    let key = opts.key_filter.clone();
    subscribe_ws(
        &opts.ws_path,
        key.as_deref(),
        move |payload_json| match serde_json::from_value::<T>(payload_json) {
            Ok(value) => {
                tracing::info!(
                    ws_path = %ws_path_for_log,
                    "[photon-leptos] received event, applying Replace"
                );
                direct_value.set(Some(value));
                trigger.update(|n| *n += 1);
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "[photon-leptos] Replace: payload deserialization failed, falling back to refetch"
                );
                direct_value.set(None);
                trigger.update(|n| *n += 1);
            }
        },
    );

    resource
}

/// Replace for `Result<T, E>` server functions: event payloads deserialize as `T`,
/// then the resource is set to `Ok(T)` without a refetch round-trip.
pub fn synced_resource_replace_result<F, Fut, T, E>(
    fetcher: F,
    opts: SyncedResourceOpts,
) -> Resource<Result<T, E>>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<T, E>> + Send + 'static,
    T: serde::Serialize + serde::de::DeserializeOwned + Send + Sync + Clone + PartialEq + 'static,
    E: Send + Sync + Clone + PartialEq + serde::Serialize + serde::de::DeserializeOwned + 'static,
{
    let trigger = RwSignal::new(0u64);
    let direct_value: RwSignal<Option<Result<T, E>>> = RwSignal::new(None);

    let resource = Resource::new(
        move || (trigger.get(), direct_value.get()),
        move |(_, injected)| {
            let fetcher_fut = fetcher();
            async move {
                if let Some(val) = injected {
                    val
                } else {
                    fetcher_fut.await
                }
            }
        },
    );

    tracing::info!(
        topic = %opts.topic,
        ws_path = %opts.ws_path,
        strategy = "Replace",
        "[photon-leptos] synced_resource (Result Ok payload)"
    );

    let ws_path_for_log = opts.ws_path.clone();
    let key = opts.key_filter.clone();
    subscribe_ws(
        &opts.ws_path,
        key.as_deref(),
        move |payload_json| match serde_json::from_value::<T>(payload_json) {
            Ok(value) => {
                tracing::info!(
                    ws_path = %ws_path_for_log,
                    "[photon-leptos] received event, applying Replace Ok"
                );
                direct_value.set(Some(Ok(value)));
                trigger.update(|n| *n += 1);
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "[photon-leptos] Replace: Ok-payload deserialization failed, falling back to refetch"
                );
                direct_value.set(None);
                trigger.update(|n| *n += 1);
            }
        },
    );

    resource
}

fn synced_resource_refetch<F, Fut, T>(fetcher: F, opts: SyncedResourceOpts) -> Resource<T>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = T> + Send + 'static,
    T: serde::Serialize + serde::de::DeserializeOwned + Send + Sync + 'static,
{
    let trigger = RwSignal::new(0u64);

    let resource = Resource::new(move || trigger.get(), move |_| fetcher());

    tracing::info!(
        topic = %opts.topic,
        ws_path = %opts.ws_path,
        strategy = "Refetch",
        "[photon-leptos] synced_resource"
    );

    let ws_path_for_log = opts.ws_path.clone();
    let key = opts.key_filter.clone();
    subscribe_ws(&opts.ws_path, key.as_deref(), move |_| {
        tracing::info!(
            ws_path = %ws_path_for_log,
            "[photon-leptos] received event, triggering refetch"
        );
        trigger.update(|n| *n += 1);
    });

    resource
}
