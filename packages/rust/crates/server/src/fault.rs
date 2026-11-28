use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use govee_toolkit::{Category, Error};
use serde_json::json;

/// The HTTP status of an SDK failure. The match reads `Error::code`, which
/// the SDK keeps stable.
pub(crate) fn status_of(error: &Error) -> StatusCode {
    match error.code() {
        _ if error.is_offline() => StatusCode::SERVICE_UNAVAILABLE,
        "shut_down" | "stream_closed" => StatusCode::SERVICE_UNAVAILABLE,
        "no_such_target" => StatusCode::NOT_FOUND,
        "target_not_understood" | "ambiguous_target" => StatusCode::BAD_REQUEST,
        "out_of_range" => StatusCode::UNPROCESSABLE_ENTITY,
        "rate_limited" => StatusCode::TOO_MANY_REQUESTS,
        // The device or the API answered, and the answer is a refusal.
        "api" | "provision_refused" => StatusCode::BAD_GATEWAY,
        // The files cannot be read: a fault of this computer.
        "config" | "env" | "local_devices" => StatusCode::INTERNAL_SERVER_ERROR,
        _ => match error.category() {
            Category::Codec => StatusCode::UNPROCESSABLE_ENTITY,
            Category::Config => StatusCode::CONFLICT,
            // `Category` is non-exhaustive.
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        },
    }
}

/// `{"error", "code"}` with the status of [`status_of`].
pub(crate) fn failed(error: &Error) -> Response {
    let body = json!({ "error": error.to_string(), "code": error.code() });
    (status_of(error), Json(body)).into_response()
}

#[cfg(test)]
mod tests {
    use govee_toolkit::DeviceId;

    use super::*;

    #[test]
    fn a_device_that_does_not_answer_is_offline_and_answers_503() {
        let error = Error::NoModeAvailable {
            id: DeviceId::new("AA:BB"),
            modes: vec![govee_toolkit::Mode::Lan],
        };
        assert!(error.is_offline());
        assert_eq!(status_of(&error), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn a_configuration_conflict_answers_409() {
        let error = Error::Configuration(Vec::new());
        assert!(!error.is_offline());
        assert_eq!(status_of(&error), StatusCode::CONFLICT);
    }
}
