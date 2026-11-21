//! Embeds the build of `apps/dashboard`, so `govee-server` ships as one file.
//!
//! In the repository the build reads `apps/dashboard/dist`. A crate from
//! crates.io reads `web/`, which the release workflow copies beside the
//! manifest. With neither, the binary carries no UI and serves the API alone.
//!
//! A build script fails the build loudly: the no-panic rule of the library
//! does not apply here.
#![allow(clippy::expect_used, clippy::panic, clippy::format_push_string)]

use std::path::{Path, PathBuf};
use std::{env, fs};

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    // packages/rust/crates/server -> the repository root.
    let dashboard = manifest
        .ancestors()
        .nth(4)
        .map(|root| root.join("apps/dashboard"));
    let dir = match dashboard {
        // The whole folder is watched, so a first build of `dist/` reruns
        // this script. A missing path would rerun it on every build.
        Some(dashboard) if dashboard.is_dir() => {
            println!("cargo:rerun-if-changed={}", dashboard.display());
            dashboard.join("dist")
        }
        _ => {
            let vendored = manifest.join("web");
            if vendored.is_dir() {
                println!("cargo:rerun-if-changed={}", vendored.display());
            }
            vendored
        }
    };
    println!("cargo:rerun-if-changed=build.rs");

    let mut files = Vec::new();
    if dir.is_dir() {
        walk(&dir, &mut files);
    }
    files.sort();

    let mut out = String::from("static ASSETS: &[(&str, &[u8])] = &[\n");
    for path in &files {
        let name = path
            .strip_prefix(&dir)
            .expect("a file under the web directory")
            .components()
            .map(|c| c.as_os_str().to_str().expect("utf-8 file name"))
            .collect::<Vec<_>>()
            .join("/");
        out.push_str(&format!(
            "    ({name:?}, include_bytes!({:?})),\n",
            path.display().to_string()
        ));
    }
    out.push_str("];\n");

    let dest = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR")).join("web.rs");
    fs::write(&dest, out).expect("write web.rs");
}

fn walk(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries =
        fs::read_dir(dir).unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            walk(&path, files);
        } else {
            files.push(path);
        }
    }
}
