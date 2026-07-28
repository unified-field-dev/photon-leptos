# photon-axum

Axum WebSocket integration for Photon browser clients — boot-time route registration and WS handlers.

**App authors** should start with the [repository README](../README.md) and `#[photon_leptos::synced]` — this crate is server wiring.

## Boot checklist

1. **Photon on app state** — implement [`HasPhoton`](src/axum_ws/state.rs) on your Axum `State` type:

   ```rust
   impl HasPhoton for AppState {
       fn photon_arc(&self) -> Arc<Photon> {
           Arc::clone(&self.photon)
       }

       // Production hosts MUST allowlist Origins (default denies all).
       // fn allow_ws_origin(&self, origin: Option<&str>) -> bool {
       //     origin == Some("https://app.example")
       // }
   }
   ```

2. **Synced routes in the binary** — ensure a linked crate uses `#[photon_leptos::synced]` so `inventory::submit!` registers [`WsRouteDescriptor`](src/axum_ws/descriptor.rs) entries.

3. **Merge WS routes at boot:**

   ```rust
   use photon_axum::{HeadlessWsAuth, ws_router};

   // Headless / demo — no user key extraction
   app = ws_router::<AppState, HeadlessWsAuth>(app);

   // Your auth newtype implements PhotonUserExtractor + FromRequestParts<S>
   // app = ws_router::<AppState, YourAuth>(app);
   ```

4. **Auth modes** — macro attribute `auth = "none"` vs `auth = "user"` selects [`WsAuthMode`](src/axum_ws/descriptor.rs). `none` allows an optional client `?key=`; `user` routes call `auth.user_key()` for partition filtering. Key-mismatch responses use a generic 403 body (raw keys are not reflected).

5. **Origin and keys** — override [`HasPhoton::allow_ws_origin`](src/axum_ws/state.rs) for cookie-authenticated deployments (default denies every Origin). Subscribe keys must be valid UTF-8 after percent-decode, must not contain control characters (`\0`, `\r`, `\n`, …), and are capped at [`MAX_KEY_LEN`](src/axum_ws/ws_query.rs) (256 bytes); malformed encodings are rejected.

6. **Host responsibilities** — enforce connection/group/rate limits, TLS, and shutdown outside this crate. Prefer [`SyncedWsConfig::try_new`](src/axum_ws/ws.rs) / [`WsFanoutMode::from_env`](src/axum_ws/ws.rs) which return [`FanoutConfigError`](src/axum_ws/ws.rs) on invalid `PHOTON_AXUM_WS_FANOUT`. Requesting `broadcast_hub` without `ws_hub()` returns **503** (no silent fallback).

## Exports

| Symbol | Role |
|--------|------|
| [`ws_router`](src/lib.rs) | Discover inventory routes and mount GET handlers |
| [`apply_ws_routes`](src/axum_ws/routes.rs) | Lower-level route registration |
| [`synced_ws_handler`](src/axum_ws/ws.rs) | Manual per-topic handler if not using inventory |
| [`SyncedWsConfig`](src/axum_ws/ws.rs) / [`try_new`](src/axum_ws/ws.rs) | Topic + key + fanout config |
| [`WsFanoutMode`](src/axum_ws/ws.rs) | `PerSubscribe` (default) or `BroadcastHub` |
| [`FanoutConfigError`](src/axum_ws/ws.rs) | Invalid env value or hub required but missing |
| [`WsBroadcastHub`](src/axum_ws/hub.rs) | Process-local shared subscribe + serialize fanout |
| [`HasPhoton`](src/axum_ws/state.rs) | Photon + optional hub + Origin policy |
| [`PhotonUserExtractor`](src/axum_ws/auth.rs) | Host auth trait for `auth = "user"` |
| [`HeadlessWsAuth`](src/axum_ws/auth.rs) | No-op auth for demos and headless servers |
| [`KeyResolveError`](src/axum_ws/key_resolve.rs) | Auth/key failures; [`client_message`](src/axum_ws/key_resolve.rs) for HTTP bodies |

## Broadcast hub (optional)

By default each WebSocket runs its own `photon.subscribe` + JSON serialize
(**per-subscribe**). For many clients on the same `(topic, key_filter)`, enable
the process-local **broadcast hub**:

1. Hold `Arc<WsBroadcastHub>` on app state and return it from [`HasPhoton::ws_hub`](src/axum_ws/state.rs).
2. Set `PHOTON_AXUM_WS_FANOUT=broadcast_hub` (or pass `SyncedWsConfig::with_fanout`).

Hub gains apply when many sockets share the same subscribe scope. Distinct
`key_filter` values (auth-scoped or `?key=`) remain **separate hub groups** —
as cardinality approaches connection count, cost approaches per-subscribe.

Group cleanup is **generation-aware**: an obsolete reader cannot delete a
replacement group created under the same key.

`BroadcastHub` is **process-local** and experimental for multi-replica fleets.

## Docs

`cargo doc -p photon-axum --features runtime --open`

## Features

| Feature | Purpose |
|---------|---------|
| `ssr` | Enable Axum WS handlers (required) |

Depends on [`photon-runtime`](https://github.com/unified-field-dev/photon) from crates.io via the workspace.
