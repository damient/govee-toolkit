//! `${frames}`: every frame of a command, in base64, inside one envelope.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;

use crate::codec::catalog::Mode;
use crate::codec::{Args, Catalog, validate};

fn catalog(commands: &str) -> Catalog {
    let source = format!(
        "schema_version: 1\nsku: HTEST\nfamily: test\nname: Test\n\
         capabilities: {{}}\ncommands:\n  lan:\n{commands}"
    );
    Catalog::from_sources([("HTEST.yaml", source.as_str())]).expect("the device file parses")
}

const CHUNKED: &str = "    upload:\n      cmd: ptReal\n\
     \n      payload: { command: \"${frames}\" }\n\
     \n      body: \"${b:bytes}\"\n      chunk:\n        size: 17\n\
     \n        header: \"A3 00 ${total} <pad:20> <xor>\"\n\
     \n        data: \"A3 ${index} ${chunk:bytes} <pad:20> <xor>\"\n\
     \n        footer: \"A3 FF <pad:20> <xor>\"\n      args:\n\
     \n        b: { type: bytes, max_len: 64 }\n";

#[test]
fn a_chunked_body_reaches_the_envelope_as_one_string_per_frame() {
    let catalog = catalog(CHUNKED);
    let device = catalog.device("HTEST").expect("the SKU resolves");
    assert_eq!(validate::device(device), []);

    let args = Args::new().bytes("b", vec![7; 20]);
    let encoded = super::encode(device, Mode::Lan, "upload", &args).expect("it encodes");
    let carried = encoded.message.expect("an envelope")["msg"]["data"]["command"].clone();
    let expected: Vec<String> = encoded.frames.iter().map(|f| BASE64.encode(f)).collect();

    assert_eq!(encoded.frames.len(), 4);
    assert_eq!(carried, serde_json::json!(expected));
}

#[test]
fn one_frame_reaches_the_envelope_as_a_list_of_one() {
    let catalog = catalog(
        "    paint:\n      cmd: ptReal\n\
         \n      payload: { command: \"${frames}\" }\n\
         \n      frame: \"33 05 ${v} <pad:20> <xor>\"\n      args:\n\
         \n        v: { type: int, range: [0, 255] }\n",
    );
    let device = catalog.device("HTEST").expect("the SKU resolves");
    assert_eq!(validate::device(device), []);

    let encoded =
        super::encode(device, Mode::Lan, "paint", &Args::new().int("v", 1)).expect("it encodes");
    let carried = &encoded.message.expect("an envelope")["msg"]["data"]["command"];
    assert_eq!(
        carried,
        &serde_json::json!([BASE64.encode(&encoded.frames[0])])
    );
}

#[test]
fn a_body_the_envelope_never_carries_is_refused() {
    let catalog = catalog(&CHUNKED.replace("\"${frames}\"", "1"));
    let device = catalog.device("HTEST").expect("the SKU resolves");
    let problems = validate::device(device);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].message.contains("`${frames}`"));
}

#[test]
fn a_list_of_exchanges_has_no_frames_to_carry() {
    let catalog = catalog(
        "    state:\n      cmd: ptReal\n\
         \n      payload: { command: \"${frames}\" }\n      frames:\n\
         \n        - send: \"AA 01 <pad:20> <xor>\"\n          reply: \"AA 01 ${lit}\"\n\
         \n      args:\n        lit: { type: int, range: [0, 1] }\n",
    );
    let device = catalog.device("HTEST").expect("the SKU resolves");
    let problems = validate::device(device);
    assert!(
        problems
            .iter()
            .any(|p| p.message.contains("neither a `frame:` nor a `body:`")),
        "{problems:?}"
    );
}
