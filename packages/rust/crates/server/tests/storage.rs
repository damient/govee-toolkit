//! `studio.json`, `/api/settings` and `/api/backup`, against a temporary
//! folder.

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use std::net::{Ipv4Addr, TcpListener};
use std::path::{Path, PathBuf};

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use govee_toolkit_server::{Options, Storage, Web, router, serve};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tower::ServiceExt;

fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("govee-server-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn options(dir: &Path) -> Options {
    Options {
        web: Web::None,
        storage: Some(Storage::in_dir(dir)),
    }
}

async fn call(options: &Options, request: Request<Body>) -> (StatusCode, Value) {
    let response = router(options).unwrap().oneshot(request).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&body).unwrap())
}

fn get(uri: &str) -> Request<Body> {
    Request::get(uri).body(Body::empty()).unwrap()
}

fn send(method: &str, uri: &str, body: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_owned()))
        .unwrap()
}

#[tokio::test]
async fn settings_start_at_the_defaults_and_survive_a_restart() {
    let dir = folder("settings");
    let (status, settings) = call(&options(&dir), get("/api/settings")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        settings,
        json!({ "theme": "dark", "language": null, "advanced": false })
    );

    let change = json!({ "theme": "light", "language": "en" }).to_string();
    let (status, settings) = call(&options(&dir), send("PUT", "/api/settings", &change)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        settings,
        json!({ "theme": "light", "language": "en", "advanced": false })
    );

    // A new router reads the file again, as a new start of the server does.
    let (_, settings) = call(&options(&dir), get("/api/settings")).await;
    assert_eq!(settings["theme"], "light");
    let file: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("studio.json")).unwrap()).unwrap();
    assert_eq!(file["server"]["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(file["studio"]["version"], "0.1.0");
    assert_eq!(file["studio"]["settings"]["language"], "en");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn a_pref_that_is_not_valid_changes_nothing() {
    let dir = folder("settings-invalid");
    for body in [
        r#"{"theme":"pink"}"#,
        r#"{"colour":1}"#,
        "[]",
        r#"{"language":"../x"}"#,
    ] {
        let (status, answer) = call(&options(&dir), send("PUT", "/api/settings", body)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        assert!(answer["error"].is_string());
    }
    assert!(!dir.join("studio.json").exists());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn a_server_with_no_storage_keeps_settings_in_memory_and_has_no_backup() {
    let options = Options {
        web: Web::None,
        storage: None,
    };
    let (status, _) = call(&options, get("/api/settings")).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(&options, get("/api/backup")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

const CONFIG: &str = "# my lights\ndevices:\n  \"AA:BB:CC:DD:EE:FF\":\n    name: desk\n";

#[tokio::test]
async fn a_backup_restores_every_file_in_another_folder() {
    let from = folder("backup-from");
    std::fs::write(from.join("config.yaml"), CONFIG).unwrap();
    std::fs::write(from.join("credentials.env"), "GOVEE_API_KEY=a-key\n").unwrap();
    let theme = json!({ "theme": "auto", "advanced": true }).to_string();
    call(&options(&from), send("PUT", "/api/settings", &theme)).await;

    let response = router(&options(&from))
        .unwrap()
        .oneshot(get("/api/backup"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let disposition = response.headers()[header::CONTENT_DISPOSITION]
        .to_str()
        .unwrap();
    assert_eq!(disposition, "attachment", "Studio names the file");
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let document: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(document["format"], "govee-toolkit-backup");
    assert_eq!(document["server"]["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(document["studio"]["version"], "0.1.0");
    assert_eq!(document["config"], CONFIG);
    assert_eq!(document["credentials"], json!({ "GOVEE_API_KEY": "a-key" }));
    assert_eq!(document["studio"]["settings"]["theme"], "auto");

    let to = folder("backup-to");
    std::fs::write(to.join("config.yaml"), "defaults:\n  modes: [ble]\n").unwrap();
    let text = document.to_string();
    let (status, answer) = call(&options(&to), send("POST", "/api/backup", &text)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(
        std::fs::read_to_string(to.join("config.yaml")).unwrap(),
        CONFIG
    );
    assert_eq!(
        std::fs::read_to_string(to.join("config.yaml.import.bak")).unwrap(),
        "defaults:\n  modes: [ble]\n"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(to.join("credentials.env"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    let again = router(&options(&to))
        .unwrap()
        .oneshot(get("/api/backup"))
        .await
        .unwrap();
    let body = again.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(serde_json::from_slice::<Value>(&body).unwrap(), document);
    std::fs::remove_dir_all(&from).unwrap();
    std::fs::remove_dir_all(&to).unwrap();
}

#[tokio::test]
async fn a_backup_with_one_bad_part_writes_nothing() {
    let dir = folder("backup-invalid");
    let documents = [
        json!({ "format": "other", "version": 1, "config": null, "studio": { "version": "0.1.0" }, "credentials": null }),
        json!({ "format": "govee-toolkit-backup", "config": "defaults:\n  mode: [lan]\n", "studio": { "version": "0.1.0" }, "credentials": null }),
        json!({ "format": "govee-toolkit-backup", "config": null, "studio": { "version": "9.0.0" }, "credentials": null }),
        json!({ "format": "govee-toolkit-backup", "config": null, "studio": { "version": "0.1.0" }, "credentials": { "GOVEE_API_KEY": "a'b" } }),
    ];
    for document in documents {
        let text = document.to_string();
        let (status, answer) = call(&options(&dir), send("POST", "/api/backup", &text)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{document}");
        assert!(answer["error"].is_string());
    }
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
    std::fs::remove_dir_all(&dir).unwrap();
}

async fn raw(addr: std::net::SocketAddr, request: &str) -> Value {
    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    serde_json::from_str(response.split("\r\n\r\n").nth(1).unwrap()).unwrap()
}

fn start(dir: &Path) -> std::net::SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let addr = listener.local_addr().unwrap();
    let options = options(dir);
    tokio::spawn(async move { serve(listener, &options).await });
    addr
}

#[tokio::test]
async fn the_share_switch_survives_a_restart() {
    let dir = folder("share");
    let addr = start(&dir);
    let body = r#"{"shared":true}"#;
    let request = format!(
        "POST /api/share HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let state = raw(addr, &request).await;
    if state["error"] == "no network address" {
        // A host with no network: nothing to share, nothing to keep.
        return;
    }
    assert_eq!(state["shared"], true, "{state}");
    let file = std::fs::read_to_string(dir.join("studio.json")).unwrap();
    assert!(file.contains("\"share\": true"), "{file}");

    let second = start(&dir);
    let request = format!("GET /api/share HTTP/1.1\r\nHost: {second}\r\nConnection: close\r\n\r\n");
    assert_eq!(raw(second, &request).await["shared"], true);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn restore_puts_back_the_files_from_before_the_import() {
    let dir = folder("restore");
    std::fs::write(dir.join("config.yaml"), CONFIG).unwrap();
    let light = json!({ "theme": "light" }).to_string();
    call(&options(&dir), send("PUT", "/api/settings", &light)).await;
    std::fs::write(dir.join("config.yaml.bak"), "a copy of the user").unwrap();
    let (status, _) = call(&options(&dir), send("POST", "/api/backup/restore", "")).await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "a .bak of the user is not an import"
    );

    let document = json!({
        "format": "govee-toolkit-backup",
        "config": "defaults:\n  modes: [ble]\n",
        "studio": { "version": "0.1.0", "settings": { "theme": "auto" } },
        "credentials": { "GOVEE_API_KEY": "a-key" },
    })
    .to_string();
    let (status, _) = call(&options(&dir), send("POST", "/api/backup", &document)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(dir.join("credentials.env").exists());

    let (status, answer) = call(&options(&dir), send("POST", "/api/backup/restore", "")).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(
        std::fs::read_to_string(dir.join("config.yaml")).unwrap(),
        CONFIG
    );
    assert!(
        !dir.join("credentials.env").exists(),
        "there was no file before"
    );
    let (_, settings) = call(&options(&dir), get("/api/settings")).await;
    assert_eq!(settings["theme"], "light");
    let left: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|entry| Some(entry.ok()?.path()))
        .filter(|path| path.to_string_lossy().ends_with(".import.bak"))
        .collect();
    assert!(left.is_empty(), "{left:?}");
    assert!(
        dir.join("config.yaml.bak").exists(),
        "the copy of the user stays"
    );
    let (status, _) = call(&options(&dir), send("POST", "/api/backup/restore", "")).await;
    assert_eq!(status, StatusCode::CONFLICT);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn a_backup_from_a_newer_server_says_to_update() {
    let dir = folder("backup-newer");
    let document = json!({
        "format": "govee-toolkit-backup",
        "server": { "version": "99.0.0" },
        "rooms": [],
        "config": null,
        "studio": { "version": "0.1.0" },
        "credentials": null,
    })
    .to_string();
    let (status, answer) = call(&options(&dir), send("POST", "/api/backup", &document)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let error = answer["error"].as_str().unwrap();
    assert!(
        error.contains("99.0.0") && error.contains("Update govee-server"),
        "{error}"
    );
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
    std::fs::remove_dir_all(&dir).unwrap();
}
