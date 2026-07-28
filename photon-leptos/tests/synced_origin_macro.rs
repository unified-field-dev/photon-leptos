//! Origin gate on real `#[synced]` codegen (not a hand-mirrored probe).

#![cfg(feature = "ssr")]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use http::StatusCode;
use leptos::prelude::Update;
use photon::Photon;
use photon_leptos::server::HasPhoton;
use photon_leptos::synced;
use tokio::sync::{Mutex, MutexGuard};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::error::Error as WsError;

static PHOTON_TEST_LOCK: Mutex<()> = Mutex::const_new(());

#[synced(
    topic = "leptos.sec.origin.macro",
    ws = "/ws/sec-origin-macro",
    auth = "none"
)]
pub async fn sec_origin_macro_probe() -> Result<(), String> {
    Ok(())
}

#[derive(Clone)]
struct DenyDefaultState {
    photon: Arc<Photon>,
}

impl HasPhoton for DenyDefaultState {
    fn photon_arc(&self) -> Arc<Photon> {
        Arc::clone(&self.photon)
    }
}

#[derive(Clone)]
struct AllowlistState {
    photon: Arc<Photon>,
}

impl HasPhoton for AllowlistState {
    fn photon_arc(&self) -> Arc<Photon> {
        Arc::clone(&self.photon)
    }

    fn allow_ws_origin(&self, origin: Option<&str>) -> bool {
        origin == Some("https://app.example")
    }
}

async fn boot_photon_locked() -> (Arc<Photon>, MutexGuard<'static, ()>) {
    let guard = PHOTON_TEST_LOCK.lock().await;
    std::env::set_var(
        "PHOTON_TRANSPORT_KEY",
        "cGhvdG9uLWRldi10cmFuc3BvcnQta2V5LTMyYnl0ZXM=",
    );
    let photon = Photon::builder()
        .auto_registry()
        .build()
        .expect("photon boot");
    photon::configure(photon.clone());
    (Arc::new(photon), guard)
}

async fn serve_macro_handler<S>(state: S) -> String
where
    S: HasPhoton + Clone + Send + Sync + 'static,
{
    let app = Router::new()
        .route(
            __photon_ws_sec_origin_macro_probe::PATH,
            axum::routing::get(__photon_ws_sec_origin_macro_probe::handler::<S>),
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });
    // Brief yield so the accept loop is ready.
    tokio::time::sleep(Duration::from_millis(10)).await;
    format!("ws://{addr}/ws/sec-origin-macro")
}

async fn connect_with_origin(url: &str, origin: Option<&str>) -> Result<(), StatusCode> {
    let mut request = url.into_client_request().expect("ws request");
    if let Some(origin) = origin {
        request
            .headers_mut()
            .insert(http::header::ORIGIN, origin.parse().expect("origin header"));
    }
    match tokio_tungstenite::connect_async(request).await {
        Ok(_) => Ok(()),
        Err(WsError::Http(response)) => Err(response.status()),
        Err(other) => panic!("unexpected ws error: {other:?}"),
    }
}

#[tokio::test]
async fn synced_macro_handler_forbidden_origin_returns_403() {
    let (photon, _guard) = boot_photon_locked().await;
    let url = serve_macro_handler(DenyDefaultState { photon }).await;

    let status = connect_with_origin(&url, Some("https://evil.example"))
        .await
        .expect_err("forbidden origin must fail handshake");
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn synced_macro_handler_missing_origin_returns_403() {
    let (photon, _guard) = boot_photon_locked().await;
    let url = serve_macro_handler(DenyDefaultState { photon }).await;

    let status = connect_with_origin(&url, None)
        .await
        .expect_err("missing origin must fail handshake");
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn synced_macro_handler_allowed_origin_upgrades() {
    let (photon, _guard) = boot_photon_locked().await;
    let url = serve_macro_handler(AllowlistState { photon }).await;

    connect_with_origin(&url, Some("https://app.example"))
        .await
        .expect("allowlisted origin must upgrade");
}
