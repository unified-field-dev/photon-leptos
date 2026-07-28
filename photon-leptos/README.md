# photon-leptos

Leptos client hooks and server re-exports for Photon realtime UI.

**Start here:** [repository README](../README.md) (hero example, Status, getting started).

**API reference:** `cargo doc -p photon-leptos --features ssr,hydrate --open`

**Integrator wiring:** [`photon-axum`](../photon-axum/README.md) for `ws_router`, auth, Origin, and fanout.

## Host responsibilities

Production hosts MUST implement `HasPhoton::allow_ws_origin` with an Origin
allowlist before mounting WebSocket routes. The default rejects all origins;
only demos and tests should explicitly opt into allow-all behavior. Subscribe
keys must not contain control characters; client logs emit path + `has_key`
only (never the `?key=` value).

## Status (0.1 experimental)

| Strategy | Contract |
|----------|----------|
| **Refetch** | Supported — server function is authoritative |
| **Replace** | Experimental — payload is `T` or `Ok` of `Result<T, E>` (`synced_resource_replace_result`) |
| **Append** | Best-effort live tail — buffers during initial load; use Refetch after reconnect for authoritative lists |

Browser WebSocket is an ephemeral invalidation / live-update channel. Prefer Refetch when exact state matters across reconnect.

## Client API map

| API | Role |
|-----|------|
| `#[synced]` / `use_<fn>()` | Macro-generated resource hook |
| `synced_resource` | Refetch or plain-`T` Replace |
| `synced_resource_replace_result` | Replace for `Result<T, E>` (Ok payload) |
| `synced_resource_append` | Best-effort list append |
| `subscribe_ws` → `PhotonWsHandle` | Raw subscription + status / last_error / close |
| `use_topic_subscription` → `PhotonSubscription` | Shared trigger + same observability signals |

## Features

| Feature | Purpose |
|---------|---------|
| `hydrate` | Browser WebSocket helpers (`leptos-use`) |
| `ssr` | Leptos SSR + server re-exports of `photon-axum` (`runtime`) + inventory routes |
