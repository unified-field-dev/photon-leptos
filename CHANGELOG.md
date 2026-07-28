# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Status / guarantees** documentation for 0.1 experimental delivery (Refetch supported; Replace experimental; Append best-effort).
- [`PhotonWsHandle`](photon-leptos) — `subscribe_ws` now returns connection `status`, `last_error`, and `close()`; [`PhotonSubscription`](photon-leptos) exposes the same signals.
- [`synced_resource_replace_result`](photon-leptos) — Replace for `Result<T, E>` server functions deserializes the event payload as `T` (macro routes automatically).
- [`HasPhoton::allow_ws_origin`](photon-axum) — WebSocket Origin policy hook.
- [`FanoutConfigError`](photon-axum) / [`SyncedWsConfig::try_new`](photon-axum) — invalid `PHOTON_AXUM_WS_FANOUT` and hub-without-state fail loudly (503 / config error).
- Subscribe key UTF-8 round-trip decode with [`MAX_KEY_LEN`](photon-axum) (256 bytes); malformed encodings rejected.
- Benchmark control-plane auth (`BENCH_CONTROL_TOKEN` / `BENCH_CONTROL_OPEN`), publish caps, and single in-flight publish run.
- CI jobs for `fmt`, `doc`, `package`, and wasm `hydrate` check; removed global GitHub token URL rewrite.
- CI jobs for [`leptos-lints`](https://github.com/leptos-rs/leptos-lints) (`cargo dylint`) on hydrate UI crates and `cargo audit`.

### Changed

- Broadcast hub group cleanup is generation-aware (obsolete readers cannot delete a replacement group).
- Append buffers events that arrive while the initial snapshot is still loading.
- Key-mismatch HTTP responses use a generic body ([`KeyResolveError::client_message`](photon-axum)); raw keys are not reflected to clients.
- `WsAuthMode::None` docs clarify optional client-selected `?key=` (not pure broadcast-only).
- `photon-leptos` path dependencies declare `version` for crates.io packaging.
- `WsFanoutMode::from_env` returns `Result` (unknown env values are errors).
- `HasPhoton::allow_ws_origin` now denies all origins by default. Production hosts
  must explicitly implement an Origin allowlist; demos and tests must explicitly
  opt into allow-all behavior.
- Workspace Clippy policy: drop blanket `pedantic`/`nursery`; enable `clippy::all`, selected pedantic lints, and restriction lints (`unwrap_used`, `expect_used`, `dbg_macro`, `print_*`, …).
- [`KeyResolveError`](photon-axum) / [`FanoutConfigError`](photon-axum) use `thiserror`.
- [`photon-leptos`](photon-leptos) client logging migrated from `log` to `tracing`; e2e demo initializes `tracing_subscriber` (SSR) and `tracing-wasm` (hydrate).
- Append initial fetch uses `spawn_local_scoped` (leptos-lints).

### Security

- Origin policy hook and key length bounds for WebSocket upgrades.
- 403 key-mismatch responses no longer echo partition identifiers.
- `cargo audit` runs on every PR/push.
- `HasPhoton::allow_ws_origin` defaults to deny-all; inventory `ws_router` and macro manual handlers share the Origin gate.
- E2E demo and bench server refuse non-loopback binds unless explicitly opted in (`PHOTON_LEPTOS_DEMO_ALLOW_INSECURE` / `PHOTON_LEPTOS_BENCH_ALLOW_NONLOCAL`).
- Production implementer checklist expanded in [`SECURITY.md`](SECURITY.md).
- Subscribe keys reject control characters (`\0`, `\r`, `\n`, …) after UTF-8/length checks; hub ops logs sanitize host-supplied keys.
- Client `subscribe_ws` logs path + `has_key` only (never the `?key=` value).
- Integration coverage for `#[synced]` codegen Origin deny-default → 403.

## [0.1.0] - 2026-07-19

### Added

- `photon-leptos` — `#[synced]` server functions, client subscription helpers, and inventory registration for Leptos apps.
- `photon-axum` — Axum WebSocket router (`ws_router`) and auth/key policy wiring for Photon topics.
- `photon-leptos-macros` — procedural macros backing `#[synced]`.
- E2E counter demo and Playwright harness under `e2e/`.
- `photon-leptos-bench` load experiments and performance study notes.

[Unreleased]: https://github.com/unified-field-dev/photon-leptos/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/unified-field-dev/photon-leptos/releases/tag/v0.1.0
