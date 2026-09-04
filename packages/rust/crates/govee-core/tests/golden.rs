//! Runs the conformance vectors in `tests/fixtures/golden/`.
//!
//! These files are the contract between implementations: the Rust core, its
//! bindings and any hand-written port must all produce the same envelope and
//! the same frame bytes for the same arguments. A port that drifts fails here
//! first.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::format_collect
)]

use std::path::{Path, PathBuf};
use std::{collections::BTreeMap, fs};

use govee_core::{ArgValue, Args, Catalog, Mode};
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
    message: serde_json::Value,
    #[serde(default)]
    frame_hex: Option<String>,
}

#[derive(Deserialize)]
struct ErrorCase {
    name: String,
    command: String,
    args: BTreeMap<String, serde_json::Value>,
    code: String,
}

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
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

/// Numbers are integers, arrays are RGB triples. The device file decides which
/// is which; this only has to carry the value across.
fn to_args(raw: &BTreeMap<String, serde_json::Value>) -> Args {
    let mut args = Args::new();
    for (name, value) in raw {
        let parsed = match value {
            serde_json::Value::Number(n) => ArgValue::Int(n.as_i64().expect("integer argument")),
            serde_json::Value::Array(items) => ArgValue::Rgb(
                items
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
                    .collect(),
            ),
            other => panic!("unsupported argument shape in a golden file: {other}"),
        };
        args.insert(name.clone(), parsed);
    }
    args
}

#[test]
fn vectors_match() {
    let catalog = Catalog::embedded().expect("embedded catalog");

    for (file, golden) in golden_files() {
        let device = catalog
            .device(&golden.sku)
            .unwrap_or_else(|e| panic!("{file}: {e}"));

        for vector in &golden.vectors {
            let encoded =
                govee_core::encode(device, golden.mode, &vector.command, &to_args(&vector.args))
                    .unwrap_or_else(|e| panic!("{file} / {}: {e}", vector.name));

            assert_eq!(
                encoded.message, vector.message,
                "{file} / {}: envelope",
                vector.name
            );

            let actual_hex = encoded
                .frame
                .as_ref()
                .map(|f| f.iter().map(|b| format!("{b:02x}")).collect::<String>());
            assert_eq!(
                actual_hex.as_deref(),
                vector.frame_hex.as_deref(),
                "{file} / {}: frame bytes",
                vector.name
            );
        }

        for case in &golden.errors {
            let result =
                govee_core::encode(device, golden.mode, &case.command, &to_args(&case.args));
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
