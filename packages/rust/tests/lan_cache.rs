//! What the `lan` cache does not give a fresh process: an entry older than
//! `forget_after`, and a match for a SKU or for `Filter::all`. The cache
//! serving a command by identity is in `transport.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use govee_toolkit::lan::{Cache, KnownDevice, Options, Transport};
use govee_toolkit::{Catalog, Config, DeviceId, Filter};
use govee_toolkit_sim::Simulator;

mod common;

use common::{SKU, id};

/// A device that the cache holds and that answers nothing.
const CACHED: &str = "11:22:33:44:55:66";

const DAY: u64 = 86_400;

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock is past the epoch")
        .as_secs()
}

/// A cache file that holds [`CACHED`], last seen `age` seconds ago.
fn cache_with(name: &str, age: u64) -> (PathBuf, Cache) {
    let directory = std::env::temp_dir().join(format!(
        "govee-toolkit-lan-cache-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("the directory is created");
    let path = directory.join("devices.json");
    let file = serde_json::json!({
        "version": 1,
        "devices": [{
            "id": CACHED,
            "ip": "192.0.2.10",
            "sku": SKU,
            "last_seen": now() - age,
        }],
    });
    std::fs::write(&path, file.to_string()).expect("the cache is written");
    let cache = Cache::load(&path).expect("the cache loads");
    (directory, cache)
}

fn ids(devices: Vec<KnownDevice>) -> Vec<DeviceId> {
    devices.into_iter().map(|device| device.id).collect()
}

async fn transport(simulator: &Simulator, cache: Cache) -> Transport {
    Transport::start(Options {
        endpoints: common::endpoints(simulator),
        refresh_interval: None,
        scan_window: Duration::from_millis(200),
        cache,
        ..Options::default()
    })
    .await
    .expect("the socket binds")
}

#[tokio::test]
async fn an_entry_past_forget_after_is_not_loaded() {
    let simulator = common::simulator(SKU).await;
    let (directory, cache) = cache_with("forgotten", 49 * DAY);
    assert_eq!(cache.len(), 1);

    let lan = transport(&simulator, cache).await;
    assert_eq!(ids(lan.devices()), Vec::<DeviceId>::new());

    let _ = std::fs::remove_dir_all(&directory);
}

#[tokio::test]
async fn a_cached_entry_is_known_before_a_scan() {
    let simulator = common::simulator(SKU).await;
    let (directory, cache) = cache_with("known", DAY);

    let lan = transport(&simulator, cache).await;
    assert_eq!(ids(lan.devices()), [DeviceId::new(CACHED)]);

    let _ = std::fs::remove_dir_all(&directory);
}

#[tokio::test]
async fn a_sku_and_every_device_leave_out_a_cached_device_that_no_scan_heard() {
    let simulator = common::simulator(SKU).await;
    let (directory, cache) = cache_with("selection", DAY);
    let lan = transport(&simulator, cache).await;
    let govee = govee_toolkit::Govee::attach(
        Config::default(),
        Catalog::embedded().expect("catalog"),
        [Arc::new(lan) as Arc<_>],
    )
    .expect("the configuration applies");

    let by_sku = govee
        .devices(Filter::targets([SKU]), None)
        .await
        .expect("the SKU resolves");
    let ids: Vec<DeviceId> = by_sku.iter().map(|h| h.id().clone()).collect();
    assert_eq!(ids, [id()]);

    let all = govee
        .devices(Filter::all(), None)
        .await
        .expect("the scan runs");
    let ids: Vec<DeviceId> = all.iter().map(|h| h.id().clone()).collect();
    assert_eq!(ids, [id()]);

    let _ = std::fs::remove_dir_all(&directory);
}
