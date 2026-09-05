//! Embeds `devices/*.yaml` into the crate.
//!
//! The catalog is compiled in, so an SDK ships as a single artifact with no
//! data directory to install. Adding a SKU therefore means rebuilding — the
//! accepted trade-off; see the crate README.
//!
//! A build script fails the build loudly: the no-panic rule of the library
//! does not apply here.
#![allow(clippy::expect_used, clippy::panic, clippy::format_push_string)]

use std::path::{Path, PathBuf};
use std::{env, fs};

fn main() {
    let devices = devices_dir();
    println!("cargo:rerun-if-env-changed=GOVEE_DEVICES_DIR");
    println!("cargo:rerun-if-changed={}", devices.display());

    let mut entries: Vec<PathBuf> = fs::read_dir(&devices)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", devices.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "yaml"))
        // schema.yaml is the reference template, not a device.
        .filter(|p| p.file_name().is_some_and(|n| n != "schema.yaml"))
        .collect();
    entries.sort();

    assert!(
        !entries.is_empty(),
        "no device files found in {}",
        devices.display()
    );

    let mut out = String::from("pub(crate) static EMBEDDED: &[(&str, &str)] = &[\n");
    for path in &entries {
        println!("cargo:rerun-if-changed={}", path.display());
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .expect("utf-8 file name");
        out.push_str(&format!(
            "    ({name:?}, include_str!({:?})),\n",
            path.display().to_string()
        ));
    }
    out.push_str("];\n");

    let dest = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR")).join("devices.rs");
    fs::write(&dest, out).expect("write devices.rs");
}

/// `GOVEE_DEVICES_DIR` wins, so a vendored copy can be used when the crate is
/// built outside the monorepo.
fn devices_dir() -> PathBuf {
    if let Some(dir) = env::var_os("GOVEE_DEVICES_DIR") {
        return PathBuf::from(dir);
    }
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    // packages/rust -> packages -> the repository root.
    let root = manifest
        .ancestors()
        .nth(2)
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let in_tree = root.join("devices");
    if in_tree.is_dir() {
        return in_tree;
    }
    // Published from crates.io the layout is flattened: `include` carries the
    // catalog along, next to the manifest.
    manifest.join("devices")
}
