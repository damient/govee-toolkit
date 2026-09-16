//! What a core failure becomes in JavaScript.
//!
//! Every failure is a JavaScript `Error` carrying `code`, the stable
//! identifier the core gives the failure, and `name`, the family it belongs
//! to. Match on `code` rather than on the message: the message is written for
//! a person and can change.

use govee_toolkit::{Category, Error};
use napi::bindgen_prelude::{Function, JsObjectValue, Object, Unknown};
use napi::{Env, JsValue};

/// The name a failure of this family carries. `Category` is
/// `#[non_exhaustive]`: a family this build does not know reaches JavaScript
/// as the base name, never as the wrong one.
fn family(error: &Error) -> &'static str {
    match error.category() {
        Category::Codec => "CodecError",
        Category::Transport => "TransportError",
        Category::Config => "ConfigError",
        _ => "GoveeError",
    }
}

/// Build one JavaScript error object.
///
/// napi carries a fixed set of status codes, so the object is built here and
/// thrown as itself: `napi::Error::from` keeps a reference to it, and the
/// thrown value is the object with its `name` and its `code`.
///
/// An environment that refuses to build the object leaves the message alone
/// and drops the two properties: a failure that cannot be reported is worse
/// than one reported with less.
fn build(env: &Env, constructor: &str, name: &str, code: &str, message: String) -> napi::Error {
    let object = |message: &str| -> napi::Result<napi::Error> {
        let global = env.get_global()?;
        let class: Function<'_, String, Unknown<'_>> = global.get_named_property(constructor)?;
        let mut object: Object<'_> = class.new_instance(message.to_owned())?.coerce_to_object()?;
        object.set_named_property("name", name)?;
        object.set_named_property("code", code)?;
        Ok(napi::Error::from(object.to_unknown()))
    };
    object(&message).unwrap_or_else(|_| napi::Error::from_reason(message))
}

/// A core failure, as the error JavaScript sees.
pub(crate) fn to_js(env: &Env, error: &Error) -> napi::Error {
    build(env, "Error", family(error), error.code(), error.to_string())
}

/// Hand a core result to JavaScript.
pub(crate) fn map<T>(env: &Env, result: Result<T, Error>) -> napi::Result<T> {
    result.map_err(|error| to_js(env, &error))
}

/// A value the binding itself refuses, before the core sees it.
pub(crate) fn value_error(env: &Env, message: impl Into<String>) -> napi::Error {
    build(
        env,
        "TypeError",
        "TypeError",
        "invalid_argument",
        message.into(),
    )
}
