//! Runs the conformance vectors in `tests/fixtures/golden/`.
//!
//! These files are the contract between implementations: the Rust core and its
//! bindings must all produce the same envelope and the same frame bytes for the
//! same arguments. An implementation that drifts fails here first.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::format_collect
)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use govee_toolkit::codec::{ArgSpec, ArgValue, Args, Catalog, Command, Mode};
use serde::Deserialize;

#[derive(Deserialize)]
struct GoldenFile {
    sku: String,
    mode: Mode,
    #[serde(default)]
    vectors: Vec<Vector>,
    #[serde(default)]
    errors: Vec<ErrorCase>,
}

#[derive(Deserialize)]
struct Vector {
    name: String,
    command: String,
    args: BTreeMap<String, serde_json::Value>,
    /// The envelope, where the mode wraps the command in one.
    #[serde(default)]
    message: Option<serde_json::Value>,
    /// One frame, for a command that sends one.
    #[serde(default)]
    frame_hex: Option<String>,
    /// Every frame, in order, for a command that sends several.
    #[serde(default)]
    frames_hex: Option<Vec<String>>,
}

impl Vector {
    /// The frames the vector expects, however it spells them.
    fn expected_frames(&self) -> Vec<String> {
        match (&self.frames_hex, &self.frame_hex) {
            (Some(frames), _) => frames.clone(),
            (None, Some(frame)) => vec![frame.clone()],
            (None, None) => Vec::new(),
        }
    }
}

#[derive(Deserialize)]
struct ErrorCase {
    name: String,
    command: String,
    args: BTreeMap<String, serde_json::Value>,
    code: String,
}

fn golden_dir() -> PathBuf {
    // packages/rust -> packages -> the repository root.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repository root")
        .join("tests/fixtures/golden")
}

fn golden_files() -> Vec<(String, GoldenFile)> {
    let mut out = Vec::new();
    let mut stack = vec![golden_dir()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).expect("golden directory").flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "json") {
                let text = fs::read_to_string(&path).expect("read golden file");
                let parsed: GoldenFile = serde_json::from_str(&text)
                    .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                out.push((path.display().to_string(), parsed));
            }
        }
    }
    assert!(
        !out.is_empty(),
        "no golden files under {}",
        golden_dir().display()
    );
    out
}

/// JSON says how a value is written; the device file says what it means.
///
/// A list of numbers is zone indices to one command and opaque bytes to
/// another, so the declared type decides. An argument no command declares —
/// which is the point of the `unknown_arg` cases — is read off its JSON shape
/// instead, and reaches the codec far enough to be refused by name.
fn to_args(spec: Option<&Command>, raw: &BTreeMap<String, serde_json::Value>) -> Args {
    let mut args = Args::new();
    for (name, value) in raw {
        let declared = spec.and_then(|command| command.args.get(name));
        let parsed = match declared {
            Some(ArgSpec::Int { .. }) => ArgValue::Int(integer(value)),
            Some(ArgSpec::RgbList { .. }) => ArgValue::Rgb(triples(value)),
            Some(ArgSpec::String { .. }) => ArgValue::Text(text(value)),
            Some(ArgSpec::Zones { .. }) => ArgValue::Zones(
                numbers(value)
                    .map(|n| u16::try_from(n).expect("a zone index"))
                    .collect(),
            ),
            Some(ArgSpec::Bytes { .. }) => ArgValue::Bytes(
                numbers(value)
                    .map(|n| u8::try_from(n).expect("a byte"))
                    .collect(),
            ),
            None => match value {
                serde_json::Value::Number(_) => ArgValue::Int(integer(value)),
                serde_json::Value::String(_) => ArgValue::Text(text(value)),
                serde_json::Value::Array(_) => ArgValue::Rgb(triples(value)),
                other => panic!("unsupported argument shape in a golden file: {other}"),
            },
        };
        args.insert(name.clone(), parsed);
    }
    args
}

fn integer(value: &serde_json::Value) -> i64 {
    value.as_i64().expect("an integer argument")
}

fn text(value: &serde_json::Value) -> String {
    value.as_str().expect("a string argument").to_owned()
}

fn numbers(value: &serde_json::Value) -> impl Iterator<Item = i64> + '_ {
    value
        .as_array()
        .expect("a list of numbers")
        .iter()
        .map(integer)
}

fn triples(value: &serde_json::Value) -> Vec<[u8; 3]> {
    value
        .as_array()
        .expect("a list of RGB triples")
        .iter()
        .map(|item| {
            let triple = item.as_array().expect("an RGB triple");
            assert_eq!(triple.len(), 3, "an RGB triple has three components");
            let mut rgb = [0u8; 3];
            for (slot, v) in rgb.iter_mut().zip(triple) {
                *slot = u8::try_from(v.as_u64().expect("a byte")).expect("a byte");
            }
            rgb
        })
        .collect()
}

#[test]
fn vectors_match() {
    let catalog = Catalog::embedded().expect("embedded catalog");

    for (file, golden) in golden_files() {
        let device = catalog
            .device(&golden.sku)
            .unwrap_or_else(|e| panic!("{file}: {e}"));

        for vector in &golden.vectors {
            let encoded = govee_toolkit::codec::encode(
                device,
                golden.mode,
                &vector.command,
                &to_args(
                    device.commands.get(golden.mode).get(&vector.command),
                    &vector.args,
                ),
            )
            .unwrap_or_else(|e| panic!("{file} / {}: {e}", vector.name));

            assert_eq!(
                encoded.message, vector.message,
                "{file} / {}: envelope",
                vector.name
            );

            let actual: Vec<String> = encoded
                .frames
                .iter()
                .map(|f| f.iter().map(|b| format!("{b:02x}")).collect::<String>())
                .collect();
            assert_eq!(
                actual,
                vector.expected_frames(),
                "{file} / {}: frame bytes",
                vector.name
            );
        }

        for case in &golden.errors {
            let result = govee_toolkit::codec::encode(
                device,
                golden.mode,
                &case.command,
                &to_args(
                    device.commands.get(golden.mode).get(&case.command),
                    &case.args,
                ),
            );
            match result {
                Ok(_) => panic!(
                    "{file} / {}: expected `{}`, got success",
                    case.name, case.code
                ),
                Err(e) => assert_eq!(e.code(), case.code, "{file} / {}: {e}", case.name),
            }
        }
    }
}

#[test]
fn every_golden_file_names_a_known_device() {
    let catalog = Catalog::embedded().expect("embedded catalog");
    for (file, golden) in golden_files() {
        assert!(
            catalog.device(&golden.sku).is_ok(),
            "{file} covers `{}`, which no device file declares",
            golden.sku
        );
    }
}

/// The reverse direction, and the one that actually keeps the vectors honest.
///
/// A command with no vector is a command an implementation can get wrong
/// without anything failing until it reaches hardware. CLAUDE.md requires one
/// per command; this is what enforces it.
#[test]
fn every_catalog_command_has_a_vector() {
    let catalog = Catalog::embedded().expect("embedded catalog");

    let mut covered: BTreeMap<(String, Mode), Vec<String>> = BTreeMap::new();
    for (_, golden) in golden_files() {
        let entry = covered
            .entry((golden.sku.to_uppercase(), golden.mode))
            .or_default();
        entry.extend(golden.vectors.iter().map(|v| v.command.clone()));
        entry.extend(golden.errors.iter().map(|e| e.command.clone()));
    }

    let mut missing = Vec::new();
    for device in catalog.devices() {
        for mode in [Mode::Lan, Mode::Ble, Mode::Cloud] {
            let vectors = covered.get(&(device.sku.to_uppercase(), mode));
            for command in device.commands.get(mode).keys() {
                if !vectors.is_some_and(|c| c.iter().any(|name| name == command)) {
                    missing.push(format!("{} / {mode} / {command}", device.sku));
                }
            }
        }
    }

    assert!(
        missing.is_empty(),
        "no conformance vector covers:\n  {}\nAdd one under tests/fixtures/golden/<mode>/<SKU>.json",
        missing.join("\n  ")
    );
}
