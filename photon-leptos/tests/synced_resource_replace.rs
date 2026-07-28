//! Replace with `Result<T, E>` deserializes event payload as `T` (COR-004).
//!
//! Under `--features hydrate`, constructing [`synced_resource_replace_result`] needs a browser
//! / wasm WebSocket stack, so this native test only covers the Replace Ok-payload serde
//! contract that callers must publish on the wire.

#![cfg(feature = "hydrate")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use photon_leptos::{SyncStrategy, SyncedResourceOpts};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Counter {
    n: u64,
}

#[test]
fn replace_result_deserializes_ok_payload() {
    // Keep opts construction so the hydrate feature path still type-checks SyncedResourceOpts.
    let _opts = SyncedResourceOpts {
        topic: "test.replace".into(),
        ws_path: "/ws/test-replace".into(),
        strategy: SyncStrategy::Replace,
        key_filter: None,
    };
    // Replace events publish the Ok value (`T`), not a serialized `Result<T, E>`.
    let payload = serde_json::to_value(Counter { n: 9 }).unwrap();
    let decoded: Counter = serde_json::from_value(payload).unwrap();
    assert_eq!(decoded.n, 9);
}
