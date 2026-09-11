//! The `cloud` transport against a stub of the documented API.
//!
//! Every test runs on the loopback: no account, no key and no internet. What
//! is asserted is the request the transport builds and the status it reads
//! back.

#![cfg(feature = "cloud")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::indexing_slicing)]

mod cloud_stub;

use std::time::Duration;

use cloud_stub::{Answer, Api};
use govee_toolkit::cloud::{Options, Transport};
use govee_toolkit::codec::{Args, Catalog, Mode};
use govee_toolkit::transport::{DeviceId, Verify};

const MAC: &str = "AA:BB:CC:DD:EE:FF";
const SKU: &str = "H61A0";

const DEVICES: &str = r#"{"code":200,"message":"success","data":[
    {"sku":"H61A0","device":"AA:BB:CC:DD:EE:FF","deviceName":"hallway"}]}"#;

const CONTROLLED: &str = r#"{"requestId":"1","code":200,"msg":"success"}"#;

const STATE: &str = r#"{"requestId":"1","code":200,"msg":"success","payload":{
    "sku":"H61A0","device":"AA:BB:CC:DD:EE:FF","capabilities":[
      {"type":"devices.capabilities.on_off","instance":"powerSwitch","state":{"value":1}},
      {"type":"devices.capabilities.range","instance":"brightness","state":{"value":40}},
      {"type":"devices.capabilities.color_setting","instance":"colorRgb",
       "state":{"value":16711680}},
      {"type":"devices.capabilities.color_setting","instance":"colorTemperatureK",
       "state":{"value":0}},
      {"type":"devices.capabilities.online","instance":"online","state":{"value":true}}]}}"#;

fn id() -> DeviceId {
    DeviceId::new(MAC)
}

fn options(api: &Api) -> Options {
    Options {
        key: "test-key".to_owned(),
        base_url: api.base_url.clone(),
        min_interval: Duration::ZERO,
        ..Options::default()
    }
}

fn encode(command: &str, args: &Args) -> govee_toolkit::codec::Encoded {
    let catalog = Catalog::embedded().expect("the embedded catalog");
    let device = catalog.device(SKU).expect("the SKU resolves");
    govee_toolkit::codec::encode(device, Mode::Cloud, command, args).expect("it encodes")
}

#[tokio::test]
async fn a_scan_lists_what_the_account_owns() {
    let api = Api::start(vec![Answer::ok(DEVICES)]).await;
    let transport = Transport::start(options(&api)).expect("the client builds");

    let found = transport.scan().await.expect("the account answers");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, id());
    assert_eq!(transport.sku(&id()).as_deref(), Some(SKU));
    assert_eq!(transport.name(&id()).as_deref(), Some("hallway"));

    let request = &api.requests()[0];
    assert_eq!(request.path, "/router/api/v1/user/devices");
    assert_eq!(request.key, "test-key");
}

#[tokio::test]
async fn a_command_carries_the_capability_the_device_file_declares() {
    let api = Api::start(vec![Answer::ok(DEVICES), Answer::ok(CONTROLLED)]).await;
    let transport = Transport::start(options(&api)).expect("the client builds");
    transport.scan().await.expect("the account answers");

    let sent = transport
        .send(
            &id(),
            &encode("power", &Args::new().int("on", 1)),
            Verify::None,
        )
        .await
        .expect("the API accepts it");
    assert_eq!(sent.mode, Mode::Cloud);
    assert_eq!(sent.cmd, "powerSwitch");

    let request = api.requests().pop().expect("the control request");
    assert_eq!(request.path, "/router/api/v1/device/control");
    assert_eq!(request.body["payload"]["sku"], SKU);
    assert_eq!(request.body["payload"]["device"], MAC);
    assert_eq!(
        request.body["payload"]["capability"],
        serde_json::json!({
            "type": "devices.capabilities.on_off",
            "instance": "powerSwitch",
            "value": 1
        })
    );
    assert!(
        request.body["requestId"].is_string(),
        "every request carries an id"
    );
}

#[tokio::test]
async fn a_state_answer_is_read_by_role_and_loses_nothing() {
    let api = Api::start(vec![Answer::ok(DEVICES), Answer::ok(STATE)]).await;
    let transport = Transport::start(options(&api)).expect("the client builds");
    transport.scan().await.expect("the account answers");

    let status = transport
        .status(&id(), &encode("status", &Args::new()))
        .await
        .expect("the API answers");

    assert_eq!(status.on, Some(true));
    assert_eq!(status.brightness, Some(40));
    assert_eq!(status.color, Some([255, 0, 0]));
    assert_eq!(status.color_temp_kelvin, Some(0));
    assert!(!status.is_white());
    assert_eq!(status.raw["online"], true, "an unclaimed capability stays");
    assert_eq!(transport.last_status(&id()), Some(status));
}

#[tokio::test]
async fn a_device_no_scan_listed_is_not_addressed() {
    let api = Api::start(vec![Answer::ok(DEVICES)]).await;
    let transport = Transport::start(options(&api)).expect("the client builds");

    let error = transport
        .send(
            &DeviceId::new("11:22:33:44:55:66"),
            &encode("power", &Args::new().int("on", 1)),
            Verify::None,
        )
        .await
        .expect_err("nothing is known under that identity");
    assert_eq!(error.code(), "unknown_device");
    assert!(api.requests().is_empty(), "no request was spent");
}

#[tokio::test]
async fn a_refused_quota_is_reported_rather_than_retried() {
    let api = Api::start(vec![Answer::ok(DEVICES), Answer::too_many()]).await;
    let transport = Transport::start(options(&api)).expect("the client builds");
    transport.scan().await.expect("the account answers");

    let error = transport
        .send(
            &id(),
            &encode("power", &Args::new().int("on", 1)),
            Verify::None,
        )
        .await
        .expect_err("the API refuses");
    assert_eq!(error.code(), "rate_limited");
    assert_eq!(api.requests().len(), 2, "it was not retried");
}

#[tokio::test]
async fn a_slot_further_away_than_the_wait_fails_rather_than_queues() {
    let api = Api::start(vec![Answer::ok(DEVICES), Answer::ok(CONTROLLED)]).await;
    let transport = Transport::start(Options {
        min_interval: Duration::from_secs(60),
        max_wait: Duration::ZERO,
        ..options(&api)
    })
    .expect("the client builds");
    transport.scan().await.expect("the account answers");

    let power = encode("power", &Args::new().int("on", 1));
    transport
        .send(&id(), &power, Verify::None)
        .await
        .expect("the first command has its slot");
    let error = transport
        .send(&id(), &power, Verify::None)
        .await
        .expect_err("the second is inside the interval");
    assert_eq!(error.code(), "rate_limited");
}

#[tokio::test]
async fn this_mode_reads_no_frames() {
    use govee_toolkit::transport::Transport as _;

    let api = Api::start(vec![Answer::ok(DEVICES)]).await;
    let transport = Transport::start(options(&api)).expect("the client builds");

    let error = transport
        .read(&id(), &encode("status", &Args::new()))
        .await
        .expect_err("there is nothing to match");
    assert_eq!(error.code(), "no_reply_layout");
}

#[tokio::test]
async fn a_key_is_the_one_thing_this_mode_cannot_start_without() {
    let error = Transport::start(Options {
        key: String::new(),
        ..Options::default()
    })
    .expect_err("no key, no mode");
    assert_eq!(error.code(), "out_of_range");
}

#[tokio::test]
async fn an_enabled_mode_with_no_key_is_a_problem_and_not_a_refusal_to_start() {
    // `set_var` needs `unsafe`, which the workspace forbids: the test asserts
    // the branch the environment selects, and both are under test.
    if std::env::var(govee_toolkit::config::KEY_ENV).is_ok_and(|key| !key.trim().is_empty()) {
        return;
    }
    let config: govee_toolkit::Config =
        serde_norway::from_str("defaults:\n  modes: [cloud]\n").expect("the configuration parses");
    let catalog = Catalog::embedded().expect("the embedded catalog");
    let govee = govee_toolkit::Govee::attach(config, catalog, []).expect("a missing key starts");

    let problems = govee.problems();
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0].message.contains("cloud") && problems[0].message.contains("credential"),
        "{}",
        problems[0]
    );
}
