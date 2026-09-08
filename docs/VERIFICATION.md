# photon-leptos verification

Re-run after code or doc changes. Leptos + Axum WebSocket integration for Photon browser
clients — covered by unit, integration, and doc gates below.

## Environment

Match [`.github/workflows/ci.yml`](../.github/workflows/ci.yml):

```bash
export CARGO_BUILD_JOBS=1
export RUSTFLAGS="-D warnings"
```

Toolchain: stable. CI also checks out sibling `unified-field-dev/photon` next to this
repo for path deps — local path patches must resolve the same way.

## PR CI parity

Required PR jobs — do not skip any when claiming local CI parity:

| CI job | Local command / notes |
|--------|------------------------|
| `fmt` | `cargo fmt --all -- --check` |
| `clippy` | `cargo clippy --workspace --all-targets --all-features -- -D warnings` |
| `leptos-lints` | dylint 6.0.1 + `nightly-2025-05-14` (+ wasm32 on that nightly); `cargo dylint --all -p photon-leptos --no-deps -- --features hydrate`; `cargo dylint --all -p photon-leptos-e2e-demo --no-deps -- --features hydrate --target wasm32-unknown-unknown` |
| `audit` | `cargo audit` |
| `test` | `cargo test -p photon-axum -p photon-leptos -p photon-leptos-macros -p photon-leptos-bench --all-features` |
| `docs` | `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps` |
| `package` | `cargo package -p photon-leptos-macros --allow-dirty --list` (and photon-axum / photon-leptos) |
| `wasm-hydrate` | `cargo check -p photon-leptos --target wasm32-unknown-unknown --features hydrate` |
| `e2e` | Node 20 + Playwright; set `PHOTON_TRANSPORT_KEY` (see e2e README / ci.yml); `cargo leptos end-to-end --project photon-leptos-e2e` |

## Unit + integration + doc (CI)

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -p photon-axum -p photon-leptos -p photon-leptos-macros -p photon-leptos-bench --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
cargo check -p photon-leptos --target wasm32-unknown-unknown --features hydrate
```

### leptos-lints (required)

```bash
# cargo install cargo-dylint --locked --version 6.0.1
# cargo install dylint-link --locked --version 6.0.1
# rustup toolchain install nightly-2025-05-14 --component rustc-dev,llvm-tools-preview
# rustup target add wasm32-unknown-unknown --toolchain nightly-2025-05-14
export CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS=fallback
cargo dylint --all -p photon-leptos --no-deps -- --features hydrate
cargo dylint --all -p photon-leptos-e2e-demo --no-deps -- --features hydrate --target wasm32-unknown-unknown
```

Narrower runs:

```bash
cargo test -p photon-leptos --all-features
cargo test -p photon-axum --features runtime
RUSTDOCFLAGS="-D warnings" cargo doc -p photon-leptos --features ssr,hydrate --no-deps
RUSTDOCFLAGS="-D warnings" cargo doc -p photon-axum --features runtime --no-deps
```

## Notes

- Workspace `missing_docs` is enforced; doc builds use `RUSTDOCFLAGS="-D warnings"`.
- E2E (`cargo leptos end-to-end`) requires `PHOTON_TRANSPORT_KEY`; see `e2e/README.md` and
  `.github/workflows/ci.yml` — required PR job `e2e`, not optional.
