#![allow(missing_docs)]
//! Boot Axum + Leptos with mem Photon and inventory WS routes.

use std::sync::Arc;

use axum::Router;
use leptos::config::get_configuration;
use leptos::prelude::*;
use leptos_axum::{file_and_error_handler, generate_route_list, LeptosRoutes};
use photon_axum::ws_router;
use quickstart_refetch::photon_boot::build_photon;
use quickstart_refetch::{ensure_loopback_bind, shell, App, AppState, CounterStore, DemoUserAuth};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let conf = get_configuration(None).map_err(|e| anyhow::anyhow!("{e}"))?;
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    ensure_loopback_bind(addr).map_err(anyhow::Error::msg)?;
    let routes = generate_route_list(App);

    let photon = build_photon()?;
    let app_state = AppState {
        leptos_options: leptos_options.clone(),
        store: Arc::new(CounterStore::default()),
        photon,
    };
    let ctx_state = app_state.clone();
    let shell_options = app_state.leptos_options.clone();

    let app = Router::new()
        .leptos_routes_with_context(
            &app_state,
            routes,
            move || {
                provide_context(ctx_state.clone());
            },
            move || shell(shell_options.clone()),
        )
        .fallback(file_and_error_handler::<AppState, _>(shell));

    let app = ws_router::<AppState, DemoUserAuth>(app).with_state(app_state);

    tracing::info!(
        %addr,
        "quickstart-refetch listening — open / and click Increment"
    );
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
