//! One request to the library endpoint.

use std::time::Duration;

use super::{Error, Scene, cache};

const LIBRARY: &str = "https://app2.govee.com/appsku/v1/light-effect-libraries";
/// Without this header the endpoint answers an empty list. 5.6.01, 7.0.0 and
/// 7.6.20 gave the same answer (2026-11-25).
const APP_VERSION: (&str, &str) = ("AppVersion", "7.6.20");

/// Fetch the library of `sku`, store it in the cache, and return its scenes.
/// The request carries no account and no key.
///
/// # Errors
///
/// [`Error::Fetch`] if the request fails or the answer is empty,
/// [`Error::Unreadable`] if the answer is not a library, and
/// [`Error::BadSku`] as for the cache file name.
pub async fn fetch(sku: &str) -> Result<Vec<Scene>, Error> {
    cache::file(sku)?;
    let failed = |reason: String| Error::Fetch {
        sku: sku.to_owned(),
        reason,
    };
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(concat!("govee-toolkit/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| failed(e.to_string()))?;
    let response = http
        // `cache::file` admits ASCII letters, digits and `-` alone, which a
        // query string carries as they are.
        .get(format!("{LIBRARY}?sku={sku}"))
        .header(APP_VERSION.0, APP_VERSION.1)
        .send()
        .await
        .map_err(|e| failed(e.to_string()))?;
    let status = response.status();
    if !status.is_success() {
        return Err(failed(format!("HTTP {status}")));
    }
    let text = response.text().await.map_err(|e| failed(e.to_string()))?;
    let scenes = super::parse(sku, &text, std::path::Path::new(LIBRARY))?;
    if scenes.is_empty() {
        return Err(failed(
            "the answer lists no scene for this model".to_owned(),
        ));
    }
    cache::store(sku, &text)
}
