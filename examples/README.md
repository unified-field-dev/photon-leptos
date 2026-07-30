# photon-leptos examples

Runnable teaching hosts for browser WebSocket refetch. Playwright lives under [`e2e/`](../e2e/). Start with the embedded refetch path; branch for secure Origin, Replace/Append, or NATS-backed Photon.

All examples need transport crypto:

```bash
export PHOTON_TRANSPORT_KEY=cGhvdG9uLWRldi10cmFuc3BvcnQta2V5LTMyYnl0ZXM=
# cargo install cargo-leptos   # once
```

`e2e/demo` is an integrator **test** harness — copy APIs from it if useful, but do not treat it as a developer demo.

---

## Canonical path

### 1. `quickstart-refetch` — mem Photon + WS refetch (first)

**Teaches:** `#[photon_leptos::synced]` refetch, auth/key isolation, and `PhotonWsHandle` status / close.

```bash
cargo leptos watch --split --project quickstart-refetch
# open http://127.0.0.1:3020/
```

**Open first:** [`quickstart-refetch/src/synced.rs`](quickstart-refetch/src/synced.rs)

**Success:** Increment bumps the counter over WS; Status → `Open`; Close → `Closed`. Auth alice vs bob (and key room-1 vs room-2) stay isolated.

**Next step:** [`secure-refetch-host`](#2-secure-refetch-host--origin--session).

---

### 2. `secure-refetch-host` — Origin + session

**Teaches:** `HasPhoton::allow_ws_origin` allowlist + session cookie → `PhotonUserExtractor` for `auth = "user"`.

```bash
cargo leptos watch --split --project secure-refetch-host
# open http://127.0.0.1:3021/
```

**Open first:** [`secure-refetch-host/src/state.rs`](secure-refetch-host/src/state.rs) → [`secure-refetch-host/src/auth.rs`](secure-refetch-host/src/auth.rs)

**Success:** after Sign in, Increment syncs on the auth-scoped WS; Origins outside the allowlist are rejected.

**Next step:** [`replace-and-append-demo`](#3-replace-and-append-demo--strategies).

---

### 3. `replace-and-append-demo` — strategies

**Teaches:** `strategy = "replace"` (payload → resource) and `strategy = "append"` (live-tail list).

```bash
cargo leptos watch --split --project replace-and-append-demo
# open http://127.0.0.1:3022/
```

**Open first:** [`replace-and-append-demo/src/synced.rs`](replace-and-append-demo/src/synced.rs)

**Success:** Bump replace updates the snapshot from the event payload; Append line grows the list.

**Next step:** [`brokered-live-ui`](#4-brokered-live-ui--nats) or back to the [root README](../README.md).

---

### 4. `brokered-live-ui` — NATS

**Teaches:** Same refetch UI with Photon on NATS JetStream. Skips cleanly when `PHOTON_NATS_URL` is unset.

```bash
docker run -d --name photon-nats -p 4222:4222 nats:2.10 -js
export PHOTON_NATS_URL=nats://127.0.0.1:4222 PHOTON_NATS_STREAM=photon PHOTON_ALLOW_INSECURE_BROKER=1
cargo leptos watch --split --project brokered-live-ui
# open http://127.0.0.1:3023/
```

**Open first:** [`brokered-live-ui/src/photon_boot.rs`](brokered-live-ui/src/photon_boot.rs)

**Success:** NATS storage boots; Increment updates the counter.

**Next step:** Photon repo brokered publisher/worker examples for split processes.

---

## Quick reference

| Example | Port | Command |
|---------|------|---------|
| `quickstart-refetch` | 3020 | `cargo leptos watch --split --project quickstart-refetch` |
| `secure-refetch-host` | 3021 | `cargo leptos watch --split --project secure-refetch-host` |
| `replace-and-append-demo` | 3022 | `cargo leptos watch --split --project replace-and-append-demo` |
| `brokered-live-ui` | 3023 | `cargo leptos watch --split --project brokered-live-ui` |

Compile-check (no WASM / no browser):

```bash
cargo check -p quickstart-refetch --features ssr
cargo check -p secure-refetch-host --features ssr
cargo check -p replace-and-append-demo --features ssr
cargo check -p brokered-live-ui --features ssr
```
