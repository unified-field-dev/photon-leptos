# Security Policy

## Supported versions

Security fixes are accepted against the latest published `0.1.x` release line of this repository's crates (`photon-leptos`, `photon-axum`, `photon-leptos-macros`).

## Reporting a vulnerability

Please **do not** open a public GitHub issue for security-sensitive reports.

Prefer one of the following:

1. **GitHub Security Advisories** — use [Report a vulnerability](https://github.com/unified-field-dev/photon-leptos/security/advisories/new) on this repository when available.
2. Contact the maintainers privately via the repository owner listed at https://github.com/unified-field-dev/photon-leptos.

Include:

- a description of the issue and its impact
- steps to reproduce or a proof of concept when possible
- affected crate names and versions

We will acknowledge receipt as soon as practical and coordinate a fix and disclosure timeline with you.

## Production wiring

photon-leptos / photon-axum expose Photon topics to browsers over WebSockets. Hosts still own authentication, topic ACLs, CSRF protection for server functions, TLS, and connection limits. Photon broker/crypto wiring lives in the **photon** repository `SECURITY.md`.

### WebSocket Origin (required)

| Check | How |
|-------|-----|
| Allowlist Origins | Override [`HasPhoton::allow_ws_origin`](photon-axum/src/axum_ws/state.rs) for every production app state |
| Default is deny | The trait default returns `false` (rejects missing and unknown Origins) |
| Inventory and macro handlers | Both `ws_router` and `#[synced]` manual handlers enforce the same Origin gate |
| Demos only | Explicit `allow_ws_origin → true` is for local demo/bench; never ship that override |

```rust,ignore
impl HasPhoton for AppState {
    fn photon_arc(&self) -> Arc<Photon> { Arc::clone(&self.photon) }

    fn allow_ws_origin(&self, origin: Option<&str>) -> bool {
        matches!(origin, Some("https://app.example.com"))
    }
}
```

Forbidden Origin upgrades return **403** with a stable message (keys are not echoed).

### Auth, cookies, keys, and mutations

| Check | How |
|-------|-----|
| `auth = "user"` routes | Implement `PhotonUserExtractor` from trusted session/JWT context |
| Cookie flags | `Secure`, `HttpOnly`, `SameSite` on session cookies |
| CSRF | Protect state-changing server functions; Origin alone is not a full CSRF strategy |
| Key policy | Subscribe keys are UTF-8, max 256 bytes after percent-decode, no control characters; mismatch responses do not echo raw keys |

### Demo, examples, and bench (do not deploy)

| Binary | Guard |
|--------|-------|
| `examples/*` teaching hosts | Loopback-only bind; quickstart / replace / brokered use allow-all Origin for teaching — copy **`secure-refetch-host`** for production Origin + session |
| `e2e/demo` | Refuses non-loopback bind unless `PHOTON_LEPTOS_DEMO_ALLOW_INSECURE=1` |
| `photon-leptos-bench` server | Refuses non-loopback bind unless `PHOTON_LEPTOS_BENCH_ALLOW_NONLOCAL=1` |

`e2e/demo` and bench intentionally use insecure defaults (allow-all Origin, demo identity cookies, open bench data plane on loopback). They are CI / lab tools only — not developer demos (see [`examples/`](examples/)).

### Host edge

Authenticate and authorize before mounting Photon WS routes. Connection, group, and rate limits are host-owned. TLS termination is host/load-balancer owned.
