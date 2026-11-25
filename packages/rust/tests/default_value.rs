//! An `int` argument with `default:`: the codec sends it where the caller
//! leaves the argument out.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use govee_toolkit::codec::{Args, Catalog, Mode, encode, validate};

/// A shared layout whose `effect` the device that includes it gives a default.
const FAMILY: &str = "schema_version: 1\nfamily: shared-still\ncommands:\n  ble:\n    \
     still:\n      frame: \"33 05 ${effect} ${level} <pad:20> <xor>\"\n      args:\n        \
     effect: { type: int, range: [0, 255] }\n        \
     level: { type: int, range: [0, 100], default: 7 }\n";

fn catalog(effect: &str) -> govee_toolkit::codec::Result<Catalog> {
    let device = format!(
        "schema_version: 1\nsku: HFIX\nfamily: test\nname: Test\ncapabilities: {{}}\n\
         include: [shared-still]\ncommands: {{}}\noverrides:\n  ble:\n    still:\n      \
         args:\n        effect: {{ default: {effect} }}\n"
    );
    Catalog::from_sources_with([("d.yaml", device.as_str())], [("f.yaml", FAMILY)])
}

fn still(args: &Args) -> govee_toolkit::codec::Result<Vec<u8>> {
    let catalog = catalog("9").expect("it loads");
    let device = catalog.device("HFIX").expect("the device is there");
    encode(device, Mode::Ble, "still", args).map(|encoded| encoded.frames.concat())
}

#[test]
fn the_codec_sends_the_default_the_caller_leaves_out() {
    let frame = still(&Args::new()).expect("both values have a default");
    assert_eq!(frame.get(..4), Some(&[0x33, 0x05, 9, 7][..]));
}

#[test]
fn a_value_from_the_caller_wins_over_the_default() {
    let frame = still(&Args::new().int("effect", 10).int("level", 8)).expect("it is in range");
    assert_eq!(frame.get(..4), Some(&[0x33, 0x05, 10, 8][..]));
}

#[test]
fn a_default_outside_its_range_is_a_file_error() {
    let catalog = catalog("300").expect("it loads");
    let device = catalog.device("HFIX").expect("the device is there");
    let problems: Vec<String> = validate::device(device)
        .into_iter()
        .map(|problem| problem.to_string())
        .collect();
    assert!(
        problems.iter().any(|p| p.contains("`default` 300")),
        "{problems:?}"
    );
}
