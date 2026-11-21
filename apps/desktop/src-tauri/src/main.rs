//! `govee-toolkit-server` in this process, and a window on its address.
//! `docs/architecture.md` says why Tauri does not serve the page.

// No console window beside the app on Windows, in a release build.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(clippy::print_stderr)]

use std::net::{Ipv4Addr, TcpListener};
use std::path::PathBuf;

use govee_toolkit_server::{Options, WEB_DIR, Web};
use tauri::{Url, WebviewUrl, WebviewWindowBuilder};

fn web() -> Web {
    if cfg!(debug_assertions) {
        Web::Dir(PathBuf::from(WEB_DIR))
    } else {
        Web::Embedded
    }
}

/// `WKWebView` has no reload shortcut, and the macOS menu has no Reload.
#[cfg(target_os = "macos")]
mod menu {
    use tauri::menu::{Menu, MenuItem, MenuItemKind, PredefinedMenuItem};
    use tauri::{App, Manager};

    const RELOAD: &str = "reload";

    pub(crate) fn install(app: &App) -> tauri::Result<()> {
        let handle = app.handle();
        let menu = Menu::default(handle)?;
        let reload = MenuItem::with_id(handle, RELOAD, "Reload", true, Some("CmdOrCtrl+R"))?;
        for item in menu.items()? {
            if let MenuItemKind::Submenu(view) = item
                && view.text()? == "View"
            {
                view.insert_items(&[&reload, &PredefinedMenuItem::separator(handle)?], 0)?;
            }
        }
        app.set_menu(menu)?;
        app.on_menu_event(|app, event| {
            if event.id() == RELOAD
                && let Some(window) = app.get_webview_window("main")
                && let Err(e) = window.reload()
            {
                eprintln!("govee-toolkit-desktop: reload: {e}");
            }
        });
        Ok(())
    }
}

fn main() {
    let result = tauri::Builder::default()
        .setup(|app| {
            #[cfg(target_os = "macos")]
            menu::install(app)?;

            let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
            let url = Url::parse(&format!("http://{}/", listener.local_addr()?))?;

            let options = Options { web: web() };
            tauri::async_runtime::spawn(async move {
                if let Err(e) = govee_toolkit_server::serve(listener, &options).await {
                    eprintln!("govee-toolkit-desktop: server: {e}");
                }
            });

            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url))
                .title("Govee Toolkit Studio")
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
