//! The HTTPS side of `cloud`: the routes, the header that authenticates them,
//! and the two answers this mode reads.
//!
//! A route is a property of the transport, the way a port is on `lan`. No
//! command name and no SKU appears here: what to write comes from
//! [`crate::codec`], and this module carries it.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde::Deserialize;

use crate::codec::Mode;
use crate::transport::error::{Error, Result};

/// Where the documented API lives.
pub const BASE_URL: &str = "https://openapi.api.govee.com";

const DEVICES: &str = "/router/api/v1/user/devices";
const CONTROL: &str = "/router/api/v1/device/control";
const STATE: &str = "/router/api/v1/device/state";
const KEY_HEADER: &str = "Govee-API-Key";

/// One device, as the account lists it.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiDevice {
    /// The model.
    pub sku: String,
    /// The identity the API addresses it by. A MAC on a device that has
    /// Wi-Fi.
    pub device: String,
    /// The name the account gave it. For logs and interfaces, never identity.
    #[serde(default, rename = "deviceName")]
    pub device_name: String,
}

/// One capability, as a state answer reports it.
#[derive(Debug, Clone, Deserialize)]
pub struct CapabilityState {
    /// Which instance answered.
    #[serde(default)]
    pub instance: String,
    /// What it reports, under `state.value`.
    #[serde(default)]
    pub state: serde_json::Value,
}

impl CapabilityState {
    /// The reported value, whatever shape it takes.
    #[must_use]
    pub fn value(&self) -> Option<&serde_json::Value> {
        self.state.get("value")
    }
}

#[derive(Deserialize)]
struct Answer<T> {
    #[serde(default)]
    code: Option<i64>,
    #[serde(default)]
    msg: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    payload: Option<T>,
    #[serde(default)]
    data: Option<T>,
}

impl<T> Answer<T> {
    /// The body's own verdict. The API answers `200` in the body of a request
    /// it refused at another level, so both are checked.
    fn into_payload(self, endpoint: &str) -> Result<Option<T>> {
        let reason = self
            .msg
            .or(self.message)
            .unwrap_or_else(|| "no message".to_owned());
        match self.code {
            Some(200) | None => Ok(self.payload.or(self.data)),
            Some(code) => Err(Error::Api {
                endpoint: endpoint.to_owned(),
                status: u16::try_from(code).unwrap_or(0),
                reason,
            }),
        }
    }
}

#[derive(Default, Deserialize)]
struct StatePayload {
    #[serde(default)]
    capabilities: Vec<CapabilityState>,
}

/// The account's HTTPS client.
pub(crate) struct Client {
    http: reqwest::Client,
    base: String,
    key: String,
    /// Makes each `requestId` distinct within one process.
    counter: AtomicU64,
}

impl std::fmt::Debug for Client {
    /// Prints no key. It must never reach a log or a bug report — see
    /// `docs/security.md`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("base", &self.base)
            .finish_non_exhaustive()
    }
}

impl Client {
    /// Build a client for one account.
    ///
    /// # Errors
    ///
    /// [`Error::Option`] if the HTTP client cannot be built.
    pub(crate) fn new(key: String, base: &str, timeout: Duration) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| Error::Option {
                field: "request_timeout".to_owned(),
                reason: e.to_string(),
            })?;
        Ok(Self {
            http,
            base: base.trim_end_matches('/').to_owned(),
            key,
            counter: AtomicU64::new(0),
        })
    }

    /// Every device the account owns.
    ///
    /// # Errors
    ///
    /// [`Error::Api`] if the API refuses, [`Error::RateLimited`] if the quota
    /// is spent, or [`Error::Io`] if the request does not complete.
    pub(crate) async fn devices(&self) -> Result<Vec<ApiDevice>> {
        let url = format!("{}{DEVICES}", self.base);
        let request = self.http.get(&url).header(KEY_HEADER, &self.key);
        let answer: Answer<Vec<ApiDevice>> = self.send(request, DEVICES).await?;
        Ok(answer.into_payload(DEVICES)?.unwrap_or_default())
    }

    /// Write one capability.
    ///
    /// `capability` is what [`crate::codec`] built: the whole
    /// `{"type":…,"instance":…,"value":…}` object.
    ///
    /// # Errors
    ///
    /// As for [`Client::devices`].
    pub(crate) async fn control(
        &self,
        sku: &str,
        device: &str,
        capability: &serde_json::Value,
    ) -> Result<()> {
        let body = serde_json::json!({
            "requestId": self.request_id(),
            "payload": { "sku": sku, "device": device, "capability": capability },
        });
        let answer: Answer<serde_json::Value> = self.post(CONTROL, &body).await?;
        answer.into_payload(CONTROL)?;
        Ok(())
    }

    /// Read every capability the device reports.
    ///
    /// # Errors
    ///
    /// As for [`Client::devices`].
    pub(crate) async fn state(&self, sku: &str, device: &str) -> Result<Vec<CapabilityState>> {
        let body = serde_json::json!({
            "requestId": self.request_id(),
            "payload": { "sku": sku, "device": device },
        });
        let answer: Answer<StatePayload> = self.post(STATE, &body).await?;
        Ok(answer
            .into_payload(STATE)?
            .map(|payload| payload.capabilities)
            .unwrap_or_default())
    }

    /// Where a request to `path` goes. Reported on events and errors.
    pub(crate) fn endpoint(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }

    async fn post<T: serde::de::DeserializeOwned + Default>(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<Answer<T>> {
        let url = self.endpoint(path);
        let request = self
            .http
            .post(&url)
            .header(KEY_HEADER, &self.key)
            .json(body);
        self.send(request, path).await
    }

    async fn send<T: serde::de::DeserializeOwned + Default>(
        &self,
        request: reqwest::RequestBuilder,
        path: &str,
    ) -> Result<Answer<T>> {
        let endpoint = self.endpoint(path);
        let response = request.send().await.map_err(|e| Error::Io {
            context: endpoint.clone(),
            source: std::io::Error::other(e.to_string()),
        })?;

        let status = response.status();
        if status.as_u16() == 429 {
            return Err(Error::RateLimited {
                mode: Mode::Cloud,
                retry_after_ms: retry_after_ms(&response),
            });
        }
        if !status.is_success() {
            return Err(Error::Api {
                endpoint,
                status: status.as_u16(),
                reason: response.text().await.unwrap_or_default(),
            });
        }
        response.json().await.map_err(|e| Error::Api {
            endpoint,
            status: status.as_u16(),
            reason: e.to_string(),
        })
    }

    fn request_id(&self) -> String {
        let n = self.counter.fetch_add(1, Ordering::Relaxed);
        let since_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or_default();
        format!("{since_epoch}-{n}")
    }
}

/// What `Retry-After` asks for, in milliseconds. Zero where the answer carries
/// none: the caller then waits on its own interval.
fn retry_after_ms(response: &reqwest::Response) -> u64 {
    response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map_or(0, |seconds| seconds.saturating_mul(1_000))
}
