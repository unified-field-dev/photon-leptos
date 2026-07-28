//! Broadcast hub fanout tests (no browser / inventory).
//!
//! Photon process globals are exclusive — tests take `PHOTON_TEST_LOCK`.

#![cfg(feature = "runtime")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use photon::topic;
use photon::Photon;
use photon_axum::axum_ws::hub::HUB_QUEUE_CAPACITY;
use photon_axum::{SyncedWsConfig, WsBroadcastHub, WsFanoutMode};
use tokio::sync::{Mutex, MutexGuard};

static PHOTON_TEST_LOCK: Mutex<()> = Mutex::const_new(());

#[topic(name = "axum.test.hub", keyed_by = "partition")]
struct HubKeyed {
    partition: String,
}

#[topic(name = "axum.test.hub.broadcast")]
struct HubBroadcast {
    seq: u64,
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

#[test]
fn fanout_mode_parse_and_env() {
    assert_eq!(
        WsFanoutMode::parse("broadcast_hub"),
        Some(WsFanoutMode::BroadcastHub)
    );
    assert_eq!(
        WsFanoutMode::parse("per_subscribe"),
        Some(WsFanoutMode::PerSubscribe)
    );
    assert_eq!(WsFanoutMode::parse("nope"), None);
    assert_eq!(WsFanoutMode::BroadcastHub.as_str(), "broadcast_hub");

    // from_env: empty / unset → PerSubscribe; unknown → Err (OPS-001).
    let prev = std::env::var("PHOTON_AXUM_WS_FANOUT").ok();
    std::env::remove_var("PHOTON_AXUM_WS_FANOUT");
    assert_eq!(
        WsFanoutMode::from_env().unwrap(),
        WsFanoutMode::PerSubscribe
    );
    std::env::set_var("PHOTON_AXUM_WS_FANOUT", "broadcast_hub");
    assert_eq!(
        WsFanoutMode::from_env().unwrap(),
        WsFanoutMode::BroadcastHub
    );
    std::env::set_var("PHOTON_AXUM_WS_FANOUT", "nope");
    assert!(WsFanoutMode::from_env().is_err());
    assert!(
        SyncedWsConfig::try_new("t", None).is_err(),
        "try_new must surface invalid PHOTON_AXUM_WS_FANOUT"
    );
    assert_eq!(WsFanoutMode::parse("hub"), Some(WsFanoutMode::BroadcastHub));
    assert_eq!(
        WsFanoutMode::parse("per-subscribe"),
        Some(WsFanoutMode::PerSubscribe)
    );
    match prev {
        Some(v) => std::env::set_var("PHOTON_AXUM_WS_FANOUT", v),
        None => std::env::remove_var("PHOTON_AXUM_WS_FANOUT"),
    }
}

#[tokio::test]
async fn hub_broadcast_reaches_all_members() {
    let (photon, _guard) = boot_photon().await;
    let hub = Arc::new(WsBroadcastHub::new());

    let mut a = hub.join(Arc::clone(&photon), "axum.test.hub.broadcast".into(), None);
    let mut b = hub.join(Arc::clone(&photon), "axum.test.hub.broadcast".into(), None);
    assert_eq!(hub.group_count(), 1);
    assert_eq!(hub.member_count("axum.test.hub.broadcast", None), 2);

    tokio::time::sleep(Duration::from_millis(100)).await;

    HubBroadcast { seq: 7 }.publish().await.expect("publish");

    let fa = tokio::time::timeout(Duration::from_secs(2), a.rx.recv())
        .await
        .expect("timeout a")
        .expect("frame a");
    let fb = tokio::time::timeout(Duration::from_secs(2), b.rx.recv())
        .await
        .expect("timeout b")
        .expect("frame b");
    assert!(fa.contains('7'), "{fa}");
    assert!(fb.contains('7'), "{fb}");
    assert_eq!(fa.as_ref(), fb.as_ref());
}

#[tokio::test]
async fn hub_keyed_groups_are_isolated() {
    let (photon, _guard) = boot_photon().await;
    let hub = Arc::new(WsBroadcastHub::new());

    let mut a = hub.join(
        Arc::clone(&photon),
        "axum.test.hub".into(),
        Some("1234".into()),
    );
    let mut b = hub.join(
        Arc::clone(&photon),
        "axum.test.hub".into(),
        Some("1235".into()),
    );
    assert_eq!(hub.group_count(), 2);

    tokio::time::sleep(Duration::from_millis(100)).await;

    HubKeyed {
        partition: "1234".into(),
    }
    .publish()
    .await
    .expect("publish");

    let fa = tokio::time::timeout(Duration::from_secs(2), a.rx.recv())
        .await
        .expect("timeout a")
        .expect("frame a");
    assert!(fa.contains("1234"), "{fa}");

    let raced = tokio::time::timeout(Duration::from_millis(300), b.rx.recv()).await;
    assert!(raced.is_err(), "key 1235 must not receive 1234 event");
}

#[tokio::test]
async fn hub_slow_client_dropped_others_continue() {
    let (photon, _guard) = boot_photon().await;
    let hub = Arc::new(WsBroadcastHub::new());

    let mut fast = hub.join(Arc::clone(&photon), "axum.test.hub.broadcast".into(), None);
    let slow = hub.join(Arc::clone(&photon), "axum.test.hub.broadcast".into(), None);
    let _slow = slow;

    tokio::time::sleep(Duration::from_millis(100)).await;

    let drain = tokio::spawn(async move {
        let mut got = 0u32;
        while tokio::time::timeout(Duration::from_millis(200), fast.rx.recv())
            .await
            .ok()
            .flatten()
            .is_some()
        {
            got += 1;
        }
        got
    });

    for seq in 0..(HUB_QUEUE_CAPACITY as u64 + 16) {
        HubBroadcast { seq }.publish().await.expect("publish");
        tokio::task::yield_now().await;
    }

    tokio::time::sleep(Duration::from_millis(200)).await;
    drain.abort();
    assert_eq!(
        hub.member_count("axum.test.hub.broadcast", None),
        1,
        "slow client should be removed; fast should remain"
    );
}

#[tokio::test]
async fn per_subscribe_direct_path_still_works() {
    let (photon, _guard) = boot_photon().await;
    let mut sub = photon.subscribe("axum.test.hub.broadcast", None, None);
    tokio::time::sleep(Duration::from_millis(50)).await;
    HubBroadcast { seq: 99 }.publish().await.expect("publish");
    let ev = tokio::time::timeout(Duration::from_secs(2), sub.next())
        .await
        .expect("timeout")
        .expect("stream")
        .expect("event");
    let json = serde_json::to_string(&ev).expect("ser");
    assert!(json.contains("99"), "{json}");
}

/// Obsolete reader cleanup must not delete a replacement group (CON-002).
#[tokio::test]
async fn hub_obsolete_reader_cannot_remove_replacement_group() {
    let (photon, _guard) = boot_photon().await;
    let hub = Arc::new(WsBroadcastHub::new());
    let topic = "axum.test.hub.broadcast";

    let first = hub.join(Arc::clone(&photon), topic.into(), None);
    let gen1 = hub
        .group_generation(topic, None)
        .expect("first group generation");
    drop(first);
    // leave() removes the empty group; allow abort to settle.
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(hub.group_count(), 0);

    let mut second = hub.join(Arc::clone(&photon), topic.into(), None);
    let gen2 = hub
        .group_generation(topic, None)
        .expect("replacement generation");
    assert_ne!(gen1, gen2, "replacement must get a new generation");

    // Simulate obsolete reader final cleanup for gen1.
    assert!(
        !hub.try_remove_generation(topic, None, gen1),
        "obsolete generation must not remove replacement"
    );
    assert_eq!(hub.group_count(), 1);
    assert_eq!(hub.group_generation(topic, None), Some(gen2));

    tokio::time::sleep(Duration::from_millis(100)).await;
    HubBroadcast { seq: 42 }.publish().await.expect("publish");
    let frame = tokio::time::timeout(Duration::from_secs(2), second.rx.recv())
        .await
        .expect("timeout")
        .expect("replacement member must still receive");
    assert!(frame.contains("42"), "{frame}");
}

/// Drop / rejoin under the same key must keep delivery to the new member.
#[tokio::test]
async fn hub_rejoin_after_empty_still_receives() {
    let (photon, _guard) = boot_photon().await;
    let hub = Arc::new(WsBroadcastHub::new());
    let topic = "axum.test.hub.broadcast";

    let first = hub.join(Arc::clone(&photon), topic.into(), None);
    drop(first);
    tokio::time::sleep(Duration::from_millis(50)).await;

    let mut second = hub.join(Arc::clone(&photon), topic.into(), None);
    tokio::time::sleep(Duration::from_millis(100)).await;
    HubBroadcast { seq: 11 }.publish().await.expect("publish");
    let frame = tokio::time::timeout(Duration::from_secs(2), second.rx.recv())
        .await
        .expect("timeout")
        .expect("frame");
    assert!(frame.contains("11"), "{frame}");
}
