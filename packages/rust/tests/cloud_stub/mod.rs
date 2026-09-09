//! A stub of the documented API, on the loopback.
//!
//! It answers the three routes the transport uses and records what it was
//! sent, so a test asserts the exact body without an account or an internet
//! connection.

#![allow(dead_code, clippy::expect_used, clippy::indexing_slicing)]

use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// One request the stub answered.
#[derive(Debug, Clone)]
pub(crate) struct Received {
    pub(crate) path: String,
    pub(crate) key: String,
    pub(crate) body: serde_json::Value,
}

/// What to answer next.
#[derive(Debug, Clone)]
pub(crate) struct Answer {
    pub(crate) status: u16,
    pub(crate) body: String,
}

impl Answer {
    pub(crate) fn ok(body: &str) -> Self {
        Self {
            status: 200,
            body: body.to_owned(),
        }
    }

    pub(crate) fn too_many() -> Self {
        Self {
            status: 429,
            body: "{\"code\":429,\"msg\":\"too many requests\"}".to_owned(),
        }
    }
}

pub(crate) struct Api {
    pub(crate) base_url: String,
    pub(crate) received: Arc<Mutex<Vec<Received>>>,
    answers: Arc<Mutex<Vec<Answer>>>,
    _task: tokio::task::JoinHandle<()>,
}

impl Api {
    /// Start on an ephemeral loopback port. Each request takes the next
    /// answer, and the last one repeats.
    pub(crate) async fn start(answers: Vec<Answer>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let port = listener.local_addr().expect("address").port();
        let received = Arc::new(Mutex::new(Vec::new()));
        let queued = Arc::new(Mutex::new(answers));

        let task = tokio::spawn({
            let received = Arc::clone(&received);
            let queued = Arc::clone(&queued);
            async move {
                while let Ok((stream, _)) = listener.accept().await {
                    serve(stream, &received, &queued).await;
                }
            }
        });

        Self {
            base_url: format!("http://127.0.0.1:{port}"),
            received,
            answers: queued,
            _task: task,
        }
    }

    pub(crate) fn requests(&self) -> Vec<Received> {
        self.received.lock().expect("the log").clone()
    }
}

async fn serve(
    mut stream: TcpStream,
    received: &Arc<Mutex<Vec<Received>>>,
    answers: &Arc<Mutex<Vec<Answer>>>,
) {
    let mut raw = Vec::new();
    let mut buffer = [0u8; 4096];
    let (head, body) = loop {
        let Ok(read) = stream.read(&mut buffer).await else {
            return;
        };
        if read == 0 {
            return;
        }
        raw.extend_from_slice(&buffer[..read]);
        let text = String::from_utf8_lossy(&raw).into_owned();
        let Some((head, body)) = text.split_once("\r\n\r\n") else {
            continue;
        };
        // One read is enough for these bodies, but not guaranteed to be.
        if body.len() >= content_length(head) {
            break (head.to_owned(), body.to_owned());
        }
    };

    record(received, &head, &body);

    let answer = {
        let mut queued = answers.lock().expect("the answers");
        if queued.len() > 1 {
            queued.remove(0)
        } else {
            queued.first().cloned().unwrap_or_else(|| Answer::ok("{}"))
        }
    };
    let response = format!(
        "HTTP/1.1 {} OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        answer.status,
        answer.body.len(),
        answer.body
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.shutdown().await;
}

fn record(received: &Arc<Mutex<Vec<Received>>>, head: &str, body: &str) {
    let mut lines = head.lines();
    let path = lines
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or_default()
        .to_owned();
    let key = lines
        .find(|line| line.to_ascii_lowercase().starts_with("govee-api-key:"))
        .and_then(|line| line.split_once(':'))
        .map(|(_, value)| value.trim().to_owned())
        .unwrap_or_default();

    received.lock().expect("the log").push(Received {
        path,
        key,
        body: serde_json::from_str(body).unwrap_or(serde_json::Value::Null),
    });
}

fn content_length(head: &str) -> usize {
    head.lines()
        .find(|line| line.to_ascii_lowercase().starts_with("content-length:"))
        .and_then(|line| line.split_once(':'))
        .and_then(|(_, value)| value.trim().parse().ok())
        .unwrap_or(0)
}
