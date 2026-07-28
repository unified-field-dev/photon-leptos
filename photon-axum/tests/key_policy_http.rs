//! HTTP probes for auth + key resolution (no browser / no WS upgrade handshake).

#![cfg(feature = "runtime")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use axum::body::Body;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::{Request, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::Router;
use futures::StreamExt;
use photon::topic;
use photon::Photon;
use photon_axum::{
    origin_from_headers, reject_origin, resolve_subscribe_key, HasPhoton, KeyResolveError,
    PhotonUserExtractor, WsAuthMode,
};
use tokio::sync::{Mutex, MutexGuard};
use tower::ServiceExt;

static PHOTON_TEST_LOCK: Mutex<()> = Mutex::const_new(());

#[topic(name = "axum.test.keyed", keyed_by = "partition")]
struct TestKeyed {
    partition: String,
}

#[derive(Clone)]
struct TestState {
    photon: Arc<Photon>,
    /// Explicit Origin allowlist entry; `None` uses the deny-safe default.
    allow_origin: Option<&'static str>,
}

impl HasPhoton for TestState {
    fn photon_arc(&self) -> Arc<Photon> {
        Arc::clone(&self.photon)
    }

    fn allow_ws_origin(&self, origin: Option<&str>) -> bool {
        self.allow_origin == origin
    }
}

#[derive(Clone)]
struct DefaultOriginState {
    photon: Arc<Photon>,
}

impl HasPhoton for DefaultOriginState {
    fn photon_arc(&self) -> Arc<Photon> {
        Arc::clone(&self.photon)
    }
}

#[derive(Clone)]
struct CookieUser(Option<String>);

impl PhotonUserExtractor for CookieUser {
    fn user_key(&self) -> Option<String> {
        self.0.clone()
    }
}

impl FromRequestParts<TestState> for CookieUser {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &TestState,
    ) -> Result<Self, Self::Rejection> {
        let key = parts
            .headers
            .get(axum::http::header::COOKIE)
            .and_then(|v| v.to_str().ok())
            .and_then(|c| {
                c.split(';').find_map(|part| {
                    part.trim()
                        .strip_prefix("e2e_user=")
                        .map(str::to_string)
                        .filter(|s| !s.is_empty())
                })
            });
        Ok(CookieUser(key))
    }
}

fn key_resolve_response(err: KeyResolveError) -> Response {
    let status = match &err {
        KeyResolveError::MissingUser => StatusCode::UNAUTHORIZED,
        KeyResolveError::KeyMismatch { .. } => StatusCode::FORBIDDEN,
    };
    (status, err.client_message().to_string()).into_response()
}

async fn probe_user_key(auth: CookieUser, uri: Uri) -> Response {
    let client_key = photon_axum::axum_ws::ws_query::client_key_from_uri(&uri);
    let user_key = auth.user_key();
    match resolve_subscribe_key(WsAuthMode::User, user_key.as_deref(), client_key.as_deref()) {
        Ok(key) => (StatusCode::NO_CONTENT, format!("key={key:?}")).into_response(),
        Err(err) => key_resolve_response(err),
    }
}

fn probe_router() -> Router<TestState> {
    Router::new()
        .route("/probe-auth", axum::routing::get(probe_user_key))
        .route(
            "/probe-origin",
            axum::routing::get(probe_origin::<TestState>),
        )
}

/// Mirrors `routes.rs` origin gate used by inventory-mounted WS handlers.
async fn probe_origin<S>(
    axum::extract::State(state): axum::extract::State<S>,
    headers: axum::http::HeaderMap,
) -> Response
where
    S: HasPhoton,
{
    if !state.allow_ws_origin(origin_from_headers(&headers)) {
        return reject_origin();
    }
    StatusCode::NO_CONTENT.into_response()
}

fn boot_photon() -> Arc<Photon> {
    std::env::set_var(
        "PHOTON_TRANSPORT_KEY",
        "cGhvdG9uLWRldi10cmFuc3BvcnQta2V5LTMyYnl0ZXM=",
    );
    let photon = Photon::builder()
        .auto_registry()
        .build()
        .expect("photon boot");
    photon::configure(photon.clone());
    Arc::new(photon)
}

async fn boot_photon_locked() -> (Arc<Photon>, MutexGuard<'static, ()>) {
    let guard = PHOTON_TEST_LOCK.lock().await;
    (boot_photon(), guard)
}

fn get_request(uri: &str, cookie: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().uri(uri).method("GET");
    if let Some(cookie) = cookie {
        builder = builder.header("cookie", cookie);
    }
    builder.body(Body::empty()).unwrap()
}

#[tokio::test]
async fn user_missing_identity_returns_401() {
    let (photon, _guard) = boot_photon_locked().await;
    let state = TestState {
        photon,
        allow_origin: None,
    };
    let app = probe_router().with_state(state);

    let response = app.oneshot(get_request("/probe-auth", None)).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn user_key_mismatch_returns_403() {
    let (photon, _guard) = boot_photon_locked().await;
    let state = TestState {
        photon,
        allow_origin: None,
    };
    let app = probe_router().with_state(state);

    let response = app
        .oneshot(get_request("/probe-auth?key=1235", Some("e2e_user=1234")))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .expect("body");
    let text = String::from_utf8_lossy(&body);
    assert!(
        !text.contains("1234") && !text.contains("1235"),
        "403 body must not reflect raw keys: {text}"
    );
    assert!(text.contains("authenticated scope"), "{text}");
}

#[tokio::test]
async fn user_matching_key_ok() {
    let (photon, _guard) = boot_photon_locked().await;
    let state = TestState {
        photon,
        allow_origin: None,
    };
    let app = probe_router().with_state(state);

    let response = app
        .oneshot(get_request("/probe-auth?key=1234", Some("e2e_user=1234")))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn user_without_client_key_uses_session_key() {
    let (photon, _guard) = boot_photon_locked().await;
    let state = TestState {
        photon,
        allow_origin: None,
    };
    let app = probe_router().with_state(state);

    let response = app
        .oneshot(get_request("/probe-auth", Some("e2e_user=1234")))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn rejected_origin_returns_403() {
    let (photon, _guard) = boot_photon_locked().await;
    let state = TestState {
        photon,
        allow_origin: Some("https://app.example"),
    };
    let app = probe_router().with_state(state);

    let request = Request::builder()
        .uri("/probe-origin")
        .method("GET")
        .header("origin", "https://evil.example")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .expect("body");
    assert_eq!(String::from_utf8_lossy(&body), "origin not allowed");
}

#[tokio::test]
async fn default_origin_policy_rejects_any_origin() {
    let (photon, _guard) = boot_photon_locked().await;
    let state = DefaultOriginState { photon };
    let app = Router::new()
        .route(
            "/probe-origin",
            axum::routing::get(probe_origin::<DefaultOriginState>),
        )
        .with_state(state);

    let request = Request::builder()
        .uri("/probe-origin")
        .method("GET")
        .header("origin", "https://evil.example")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn allowed_origin_proceeds() {
    let (photon, _guard) = boot_photon_locked().await;
    let state = TestState {
        photon,
        allow_origin: Some("https://app.example"),
    };
    let app = probe_router().with_state(state);

    let request = Request::builder()
        .uri("/probe-origin")
        .method("GET")
        .header("origin", "https://app.example")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn keyed_publish_reaches_matching_subscriber_only() {
    let (photon, _guard) = boot_photon_locked().await;

    let mut sub_a = photon.subscribe("axum.test.keyed", Some("1234"), None);
    let mut sub_b = photon.subscribe("axum.test.keyed", Some("1235"), None);
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    TestKeyed {
        partition: "1234".into(),
    }
    .publish()
    .await
    .expect("publish 1234");

    let ev_a = tokio::time::timeout(std::time::Duration::from_secs(2), sub_a.next())
        .await
        .expect("timeout a")
        .expect("stream a")
        .expect("event a");
    let payload = serde_json::to_string(&ev_a).expect("serialize event");
    assert!(payload.contains("1234"), "{payload}");

    let raced = tokio::time::timeout(std::time::Duration::from_millis(300), sub_b.next()).await;
    assert!(raced.is_err(), "subscriber B must not receive 1234 publish");
}
