# photon-leptos E2E

**Status:** implemented — browser tests under `e2e/tests/` exercise a minimal Leptos + Axum harness in `e2e/demo/`.

For **developer demos** (copy-paste teaching hosts), see [`examples/`](../examples/). This directory is the Playwright e2e surface (`e2e/demo` is the harness host).

## Goal

Prove **publish → WebSocket → Leptos refetch**, including **auth / key isolation** (both directions), with a self-contained demo.

## Layout

```
e2e/
  README.md       # this file
  demo/           # Leptos + Axum workspace member (in-process mem Photon)
  tests/          # Playwright specs + worker-scoped namespace fixture
```

## Per-worker isolation (not global)

| Layer | Mechanism |
|-------|-----------|
| Server | `CounterStore` broadcast + partition counters keyed by Playwright `namespace` |
| Playwright | Worker fixture: `namespace = \`${project.name}-w${workerIndex}\`` |
| Cookies | `e2e_ns`, optional `e2e_user`, optional `e2e_key` for server-fn refetch |
| Hygiene | `beforeEach`: `POST /api/counter/reset { namespace }` |

## Demo server

**Not for production.** The demo uses insecure defaults (allow-all Origin, query-param identity cookies). It binds loopback only unless `PHOTON_LEPTOS_DEMO_ALLOW_INSECURE=1`. See [`SECURITY.md`](../SECURITY.md).

| Piece | Description |
|-------|-------------|
| Photon boot | In-process `mem` via `Photon::builder().auto_registry().build()` |
| Auth | `E2eUserAuth` reads cookie `e2e_user` → `PhotonUserExtractor::user_key` |
| Router | `ws_router::<AppState, E2eUserAuth>(app)` |
| Broadcast topic | `counter.updated` → `/ws/counter` (`auth = none`) |
| Keyed topic | `counter.keyed.updated` (`keyed_by = partition`) |
| Bind guard | Non-loopback requires `PHOTON_LEPTOS_DEMO_ALLOW_INSECURE=1` |

### Pages

| Path | Mode |
|------|------|
| `/?ns=` | Broadcast happy / sad (`mode=no-ws`, etc.) |
| `/auth-only?ns=&user=` | `auth = user`, no client `?key=` |
| `/key-only?ns=&key=` | `auth = none` + client `?key=` |
| `/auth-key?ns=&user=&key=` | `auth = user` + matching client `?key=` |

### Test-only API

| Route | Purpose |
|-------|---------|
| `POST /api/counter/increment` | Broadcast bump + publish |
| `POST /api/counter/increment-keyed` | Partition bump + keyed publish `{ namespace, partition }` |
| `POST /api/counter/reset` | Reset namespace counters/flags |
| `POST /api/e2e/scenario` | `fail_read` / `fail_publish` |

### Keyed isolation rule

Every keyed happy path opens **two** live tabs with different scopes, publishes one partition, asserts **recipient updates and peer stays put**, then publishes the other partition and asserts the **reverse**.

## Running locally

Requires `PHOTON_TRANSPORT_KEY` (Photon 0.1.1+ fail-closed crypto):

```bash
export PHOTON_TRANSPORT_KEY=cGhvdG9uLWRldi10cmFuc3BvcnQta2V5LTMyYnl0ZXM=
cd e2e/tests && npm ci && npx playwright install --with-deps
cargo leptos end-to-end --project photon-leptos-e2e   # from workspace root
```

## Playwright specs

| File | Coverage |
|------|----------|
| `counter.happy.spec.ts` | Broadcast publish → WS → refetch; second tab sync |
| `counter.sad.spec.ts` | Server error, publish failure, no-ws, WS disconnect/reload |
| `counter.keyed.spec.ts` | Auth-only / key-only / auth+key isolation (both directions); mismatch + missing identity |

## CI

The `e2e` job in [`.github/workflows/ci.yml`](../.github/workflows/ci.yml) sets `PHOTON_TRANSPORT_KEY` and runs `cargo leptos end-to-end`.
