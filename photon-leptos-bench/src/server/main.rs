#![allow(missing_docs)]
//! Standalone bench server binary.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::print_stdout,
    clippy::print_stderr
)]

use std::net::SocketAddr;

use photon_axum::WsFanoutMode;
use photon_leptos_bench::server::{
    build_photon, build_router, ensure_bench_bind_allowed, BenchState,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr: SocketAddr = std::env::var("BENCH_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8080".into())
        .parse()?;
    ensure_bench_bind_allowed(addr).map_err(anyhow::Error::msg)?;

    // Loopback demos: open control plane unless a token is configured (SEC-005).
    if addr.ip().is_loopback() && std::env::var("BENCH_CONTROL_TOKEN").is_err() {
        std::env::set_var("BENCH_CONTROL_OPEN", "1");
    }

    let mode = std::env::var("BENCH_WS_MODE")
        .ok()
        .and_then(|s| WsFanoutMode::parse(&s))
        .unwrap_or(WsFanoutMode::PerSubscribe);
    std::env::set_var("PHOTON_AXUM_WS_FANOUT", mode.as_str());

    let photon = build_photon().await?;
    let state = BenchState::new(photon, mode);
    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    eprintln!(
        "photon-leptos-bench-server listening on http://{addr} ws_mode={} control={}",
        mode.as_str(),
        if std::env::var("BENCH_CONTROL_TOKEN").is_ok() {
            "token"
        } else if std::env::var("BENCH_CONTROL_OPEN").ok().as_deref() == Some("1") {
            "open"
        } else {
            "locked"
        }
    );
    axum::serve(listener, app).await?;
    Ok(())
}
