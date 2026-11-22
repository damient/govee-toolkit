//! `/api/share`, in process and through a real listener.

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use std::net::{Ipv4Addr, TcpListener};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use govee_toolkit_server::{Options, Web, router, serve};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tower::ServiceExt;

async fn call(request: Request<Body>) -> (StatusCode, Value) {
    let response = router(&Options {
        web: Web::None,
        ..Options::default()
    })
    .unwrap()
    .oneshot(request)
    .await
    .unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&body).unwrap())
}

fn post(body: &Value) -> Request<Body> {
    Request::post("/api/share")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[tokio::test]
async fn a_router_out_of_serve_reports_sharing_off_and_no_control() {
    let (status, state) = call(Request::get("/api/share").body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        state,
        json!({ "fixed": false, "shared": false, "url": null, "control": false, "local": true })
    );
}

#[tokio::test]
async fn a_router_out_of_serve_cannot_share() {
    let (status, answer) = call(post(&json!({ "shared": true }))).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(answer["error"], "this server cannot share");
}

#[tokio::test]
async fn a_body_without_shared_is_a_bad_request() {
    let (status, _) = call(post(&json!({ "on": true }))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_loopback_client_of_serve_controls_the_switch() {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let addr = listener.local_addr().unwrap();
    let options = Options {
        web: Web::None,
        ..Options::default()
    };
    tokio::spawn(async move { serve(listener, &options).await });

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let request = format!("GET /api/share HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    let body = response.split("\r\n\r\n").nth(1).unwrap();
    let state: Value = serde_json::from_str(body).unwrap();
    assert_eq!(state["control"], true);
    assert_eq!(state["shared"], false);
}
