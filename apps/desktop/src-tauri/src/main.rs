//! The desktop app: `govee-toolkit-server` in the same process, and a window
//! on the address it serves.
//!
//! The window loads the page from the server, so the page and the API share
//! one origin. A page served by Tauri runs on `tauri://localhost`, and
//! `WKWebView` refuses its requests to `http://127.0.0.1`.

// No console window beside the app on Windows, in a release build.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// A binary reports and exits.
#![allow(clippy::print_stderr)]

use std::net::{Ipv4Addr, TcpListener};
use std::path::PathBuf;

use govee_toolkit_server::{Options, Web};
use tauri::{Url, WebviewUrl, WebviewWindowBuilder};

/// A release build serves the UI that `govee-toolkit-server` compiles in. A
/// debug build reads `apps/dashboard/dist`, so a rebuild of the UI shows on a
/// reload.
fn web() -> Web {
    if cfg!(debug_assertions) {
        Web::Dir(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../dashboard/dist"))
    } else {
        Web::Embedded
    }
}

fn main() {
    let result = tauri::Builder::default()
        .setup(|app| {
            let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
            listener.set_nonblocking(true)?;
            let url = Url::parse(&format!("http://{}/", listener.local_addr()?))?;

            let options = Options { web: web() };
            tauri::async_runtime::spawn(async move {
                let served = match tokio::net::TcpListener::from_std(listener) {
                    Ok(listener) => govee_toolkit_server::serve(listener, &options).await,
                    Err(e) => Err(e),
                };
                if let Err(e) = served {
                    eprintln!("govee-toolkit-desktop: server: {e}");
                }
            });

            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url))
                .title("govee-toolkit")
                .inner_size(960.0, 720.0)
                .min_inner_size(400.0, 480.0)
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!());
    if let Err(e) = result {
        eprintln!("govee-toolkit-desktop: {e}");
        std::process::exit(1);
    }
}
