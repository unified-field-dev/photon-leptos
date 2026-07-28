//! Client Close must clear hub membership without waiting for a publish.

#![cfg(feature = "runtime")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::WebSocketUpgrade;
use axum::extract::State;
use axum::routing::get;
use axum::Router;
use futures::SinkExt;
use photon::Photon;
use photon_axum::{synced_ws_handler, HasPhoton, SyncedWsConfig, WsBroadcastHub, WsFanoutMode};
use tokio::sync::{Mutex, MutexGuard};
use tokio_tungstenite::tungstenite::protocol::Message;

static PHOTON_TEST_LOCK: Mutex<()> = Mutex::const_new(());

#[derive(Clone)]
struct TestState {
    photon: Arc<Photon>,
    hub: Option<Arc<WsBroadcastHub>>,
}

impl HasPhoton for TestState {
    fn photon_arc(&self) -> Arc<Photon> {
        Arc::clone(&self.photon)
    }

    fn ws_hub(&self) -> Option<Arc<WsBroadcastHub>> {
        self.hub.clone()
    }
}

async fn boot_photon() -> (Arc<Photon>, MutexGuard<'static, ()>) {
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

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<TestState>,
) -> axum::response::Response {
    let config = SyncedWsConfig::new("axum.test.ws.disconnect", None)
        .with_fanout(WsFanoutMode::BroadcastHub);
    synced_ws_handler(ws, state.photon_arc(), state.ws_hub(), config).await
}

#[tokio::test]
async fn client_close_clears_hub_membership_without_publish() {
    let (photon, _guard) = boot_photon().await;
    let hub = Arc::new(WsBroadcastHub::new());
    let state = TestState {
        photon,
        hub: Some(Arc::clone(&hub)),
    };

    let app = Router::new()
        .route("/ws/disconnect-test", get(ws_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });

    let url = format!("ws://{addr}/ws/disconnect-test");
    let (mut ws, _) = tokio_tungstenite::connect_async(&url)
        .await
        .expect("ws connect");

    let topic = "axum.test.ws.disconnect";
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if hub.member_count(topic, None) == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("member joined");

    ws.send(Message::Close(None)).await.expect("close");
    let _ = ws.close(None).await;
    drop(ws);

    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if hub.member_count(topic, None) == 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("membership cleared after client Close without publish");
}

#[tokio::test]
async fn broadcast_hub_without_hub_returns_503() {
    let (photon, _guard) = boot_photon().await;
    let state = TestState { photon, hub: None };

    let app = Router::new()
        .route(
            "/ws/no-hub",
            get(
                |ws: WebSocketUpgrade, State(state): State<TestState>| async move {
                    let config = SyncedWsConfig::new("axum.test.ws.disconnect", None)
                        .with_fanout(WsFanoutMode::BroadcastHub);
                    synced_ws_handler(ws, state.photon_arc(), state.ws_hub(), config).await
                },
            ),
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });

    let url = format!("http://{addr}/ws/no-hub");
    // WebSocket upgrade without hub should fail before upgrade with 503.
    let result = tokio_tungstenite::connect_async(format!("ws://{addr}/ws/no-hub")).await;
    assert!(
        result.is_err(),
        "connect must fail when BroadcastHub has no hub; url={url}"
    );
}
