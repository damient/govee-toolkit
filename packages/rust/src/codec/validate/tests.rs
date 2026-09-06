//! Structural checks, exercised against inline device files.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use super::*;
use crate::codec::Catalog;
use crate::codec::catalog::ArgRole;

fn device_file(mode: Mode, commands: &str) -> String {
    format!(
        "schema_version: 1\nsku: HTEST\nfamily: test\nname: Test\n\
         capabilities: {{}}\ncommands:\n  {mode}:\n{commands}"
    )
}

fn parse(commands: &str) -> Catalog {
    parse_mode(Mode::Lan, commands)
}

fn parse_mode(mode: Mode, commands: &str) -> Catalog {
    Catalog::from_sources([("HTEST.yaml", device_file(mode, commands).as_str())])
        .expect("the device file parses")
}

#[test]
fn one_command_may_report_state() {
    let catalog =
        parse("    status:\n      cmd: devStatus\n      documented: true\n      role: status\n");
    let device = catalog.device("HTEST").expect("the SKU resolves");
    assert_eq!(device.status_command(Mode::Lan), Some("status"));
    assert!(super::device(device).is_empty());
}

#[test]
fn a_file_may_report_no_state_at_all() {
    let catalog = parse("    power:\n      cmd: turn\n      documented: true\n");
    let device = catalog.device("HTEST").expect("the SKU resolves");
    assert_eq!(device.status_command(Mode::Lan), None);
    assert!(super::device(device).is_empty());
}

/// A `segment_color` command whose `colors` is spelled the file's way. The
/// role is what the SDK reads; the spelling is the file's business.
const PAINT: &str = "    paint:\n      cmd: razer\n      documented: true\n\
     \n      role: segment_color\n      args:\n\
     \n        pixels: { type: rgb_list, role: colors }\n";

#[test]
fn an_argument_the_sdk_fills_is_found_by_its_role() {
    let catalog = parse(PAINT);
    let device = catalog.device("HTEST").expect("the SKU resolves");
    let command = device.commands.lan.get("paint").expect("the entry parses");
    assert_eq!(command.arg_for(ArgRole::Colors), Some("pixels"));
    assert_eq!(command.arg_for(ArgRole::Gradient), None);
    assert!(super::device(device).is_empty());
}

#[test]
fn a_role_command_missing_its_argument_leaves_nothing_to_fill() {
    let catalog = parse(
        "    paint:\n      cmd: razer\n      documented: true\n\
         \n      role: segment_color\n      args:\n\
         \n        pixels: { type: rgb_list }\n",
    );
    let device = catalog.device("HTEST").expect("the SKU resolves");
    let problems = super::device(device);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].message.contains("role: colors"));
}

#[test]
fn two_arguments_claiming_one_role_leave_nothing_to_pick() {
    let catalog = parse(
        "    paint:\n      cmd: razer\n      documented: true\n\
         \n      role: segment_color\n      args:\n\
         \n        pixels: { type: rgb_list, role: colors }\n\
         \n        zones: { type: rgb_list, role: colors }\n",
    );
    let device = catalog.device("HTEST").expect("the SKU resolves");
    let problems = super::device(device);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].message.contains("pixels, zones"));
}

#[test]
fn an_argument_role_is_fixed_to_one_type() {
    let catalog = parse(
        "    arm:\n      cmd: razer\n      documented: true\n\
         \n      role: segment_enable\n      args:\n\
         \n        armed: { type: rgb_list, role: enable }\n",
    );
    let device = catalog.device("HTEST").expect("the SKU resolves");
    let problems = super::device(device);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].message.contains("an integer"));
}

#[test]
fn the_names_the_codec_fills_in_may_not_be_declared() {
    let catalog = parse(
        "    send:\n      cmd: razer\n      documented: true\n      args:\n\
         \n        index: { type: int, range: [0, 1] }\n",
    );
    let device = catalog.device("HTEST").expect("the SKU resolves");
    let problems = super::device(device);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].message.contains("`index`"));
}

#[test]
fn a_body_needs_the_chunk_that_splits_it() {
    let catalog = parse(
        "    send:\n      cmd: razer\n      documented: true\n\
         \n      body: \"${b:bytes}\"\n      args:\n\
         \n        b: { type: bytes }\n",
    );
    let device = catalog.device("HTEST").expect("the SKU resolves");
    let problems = super::device(device);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].message.contains("no `chunk:`"));
}

#[test]
fn a_command_sends_one_frame_or_a_chunked_body_but_not_both() {
    let catalog = parse(
        "    send:\n      cmd: razer\n      documented: true\n\
         \n      frame: \"BB ${on}\"\n      payload: { pt: \"${frame}\" }\n\
         \n      body: \"${on}\"\n      chunk:\n        size: 16\n\
         \n        header: \"A1 ${count} <pad:20> <xor>\"\n\
         \n        data: \"A1 ${index} ${chunk:bytes} <pad:20> <xor>\"\n\
         \n        footer: \"A1 FF <pad:20> <xor>\"\n      args:\n\
         \n        on: { type: int, range: [0, 1] }\n",
    );
    let device = catalog.device("HTEST").expect("the SKU resolves");
    let problems = super::device(device);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].message.contains("`frame:` and `body:`"));
}

#[test]
fn an_undocumented_command_points_at_the_protocol_notes_for_its_own_mode() {
    let catalog = parse(
        "    raw:\n      cmd: status\n      documented: false\n\
         \n      notes: \"See docs/protocol/ble.md.\"\n",
    );
    let device = catalog.device("HTEST").expect("the SKU resolves");
    let problems = super::device(device);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].message.contains("docs/protocol/lan.md"));
}

#[test]
fn two_claimants_leave_nothing_to_pick() {
    let catalog = parse(
        "    status:\n      cmd: devStatus\n      documented: true\n      role: status\n\
         \n    other:\n      cmd: status\n      documented: true\n      role: status\n",
    );
    let device = catalog.device("HTEST").expect("the SKU resolves");
    let problems = super::device(device);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].message.contains("role: status"));
}

#[test]
fn a_reply_capturing_something_undeclared_has_nowhere_to_put_it() {
    let catalog = parse(
        "    state:\n      documented: true\n      cmd: raw\n\
         \n      payload: { pt: \"${frame}\" }\n\
         \n      frame: \"AA 01 <pad:20> <xor>\"\n\
         \n      reply: \"AA 01 ${lit}\"\n      args: {}\n",
    );
    let device = catalog.device("HTEST").expect("the SKU resolves");
    let problems = super::device(device);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].message.contains("`lit`"));
}

#[test]
fn an_envelope_that_never_names_its_frame_is_refused() {
    let catalog = parse(
        "    state:\n      documented: true\n      cmd: raw\n\
         \n      payload: { pt: \"AA\" }\n\
         \n      frame: \"AA 01 <pad:20> <xor>\"\n      args: {}\n",
    );
    let device = catalog.device("HTEST").expect("the SKU resolves");
    let problems = super::device(device);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0]
            .message
            .contains("never carries with `${frame}`")
    );
}

#[test]
fn a_frame_on_its_own_wire_needs_no_envelope_to_name_it() {
    let catalog = parse_mode(
        Mode::Ble,
        "    state:\n      documented: true\n\
         \n      frame: \"AA 01 <pad:20> <xor>\"\n      args: {}\n",
    );
    let device = catalog.device("HTEST").expect("the SKU resolves");
    assert!(super::device(device).is_empty());
}

#[test]
fn a_frame_on_a_wire_that_wraps_it_still_needs_an_envelope() {
    // `lan` carries every frame inside a JSON envelope, so a frame with no
    // `cmd:` to name it would go out as nothing at all.
    let catalog = parse(
        "    state:\n      documented: true\n\
         \n      frame: \"AA 01 <pad:20> <xor>\"\n      args: {}\n",
    );
    let device = catalog.device("HTEST").expect("the SKU resolves");
    let problems = super::device(device);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0]
            .message
            .contains("never carries with `${frame}`")
    );
}

#[test]
fn a_reply_layout_only_matches_bytes() {
    let catalog = parse(
        "    state:\n      documented: true\n\
         \n      frame: \"AA 01 <pad:20> <xor>\"\n\
         \n      reply: \"AA 01 ${lit} <xor>\"\n      args:\n\
         \n        lit: { type: int, range: [0, 1] }\n",
    );
    let device = catalog.device("HTEST").expect("the SKU resolves");
    let problems = super::device(device);
    assert!(
        problems.iter().any(|p| p.message.contains("only matches")),
        "{problems:?}"
    );
}

#[test]
fn a_reply_needs_a_frame_to_ask_for_it() {
    let catalog = parse(
        "    state:\n      cmd: devStatus\n      documented: true\n\
         \n      reply: \"AA 01 ${lit}\"\n      args:\n\
         \n        lit: { type: int, range: [0, 1] }\n",
    );
    let device = catalog.device("HTEST").expect("the SKU resolves");
    let problems = super::device(device);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].message.contains("no `frame:`"));
}

#[test]
fn a_command_sends_one_frame_or_a_list_of_exchanges_but_not_both() {
    let catalog = parse(
        "    state:\n      documented: true\n\
         \n      frame: \"AA 01 <pad:20> <xor>\"\n\
         \n      payload: { pt: \"${frame}\" }\n      cmd: raw\n\
         \n      frames:\n\
         \n        - send: \"AA 04 <pad:20> <xor>\"\n\
         \n          reply: \"AA 04 ${level}\"\n      args:\n\
         \n        level: { type: int, range: [0, 100] }\n",
    );
    let device = catalog.device("HTEST").expect("the SKU resolves");
    let problems = super::device(device);
    assert!(
        problems
            .iter()
            .any(|p| p.message.contains("`frames:` beside")),
        "{problems:?}"
    );
}

#[test]
fn a_step_with_no_reply_would_be_written_but_never_read() {
    let catalog = parse(
        "    state:\n      documented: true\n\
         \n      frames:\n\
         \n        - send: \"AA 01 <pad:20> <xor>\"\n\
         \n        - send: \"AA 04 <pad:20> <xor>\"\n\
         \n          reply: \"AA 04 ${level}\"\n      args:\n\
         \n        level: { type: int, range: [0, 100] }\n",
    );
    let device = catalog.device("HTEST").expect("the SKU resolves");
    let problems = super::device(device);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].message.contains("step 0"), "{problems:?}");
}

#[test]
fn two_captured_fields_claiming_one_role_leave_nothing_to_pick() {
    let catalog = parse(
        "    state:\n      documented: true\n      cmd: raw\n\
         \n      payload: { pt: \"${frame}\" }\n\
         \n      frame: \"AA 01 <pad:20> <xor>\"\n\
         \n      reply: \"AA 01 ${lit} ${also}\"\n      args:\n\
         \n        lit: { type: int, range: [0, 1], role: \"on\" }\n\
         \n        also: { type: int, range: [0, 1], role: \"on\" }\n",
    );
    let device = catalog.device("HTEST").expect("the SKU resolves");
    let problems = super::device(device);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].message.contains("role: on"));
}
