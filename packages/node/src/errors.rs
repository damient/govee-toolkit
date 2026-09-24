//! What a core failure becomes in JavaScript: an `Error` carrying `code`, the
//! stable identifier the core gives the failure, and `name`, its family.
//! Match on `code`; the message is written for a person and can change.

use govee_toolkit::Error;
use napi::bindgen_prelude::{Function, JsObjectValue, Object, Unknown};
use napi::{Env, JsValue};

fn object<'env>(
    env: &'env Env,
    constructor: &str,
    name: &str,
    code: &str,
    message: &str,
) -> napi::Result<Object<'env>> {
    let global = env.get_global()?;
    let class: Function<'_, String, Unknown<'_>> = global.get_named_property(constructor)?;
    let mut object: Object<'_> = class.new_instance(message.to_owned())?.coerce_to_object()?;
    object.set_named_property("name", name)?;
    object.set_named_property("code", code)?;
    Ok(object)
}

/// Build one JavaScript error to throw.
///
/// napi carries a fixed set of status codes, so the object is thrown as
/// itself through `napi::Error::from`. An environment that refuses to build
/// it falls back to the message alone: a failure reported with less beats one
/// not reported.
fn build(env: &Env, constructor: &str, name: &str, code: &str, message: String) -> napi::Error {
    object(env, constructor, name, code, &message).map_or_else(
        |_| napi::Error::from_reason(message),
        |object| napi::Error::from(object.to_unknown()),
    )
}

/// The name, the code and the message of a core failure. The object itself
/// is built on the JavaScript thread.
#[derive(Debug, Clone)]
pub(crate) struct Parts {
    name: &'static str,
    code: &'static str,
    message: String,
}

impl Parts {
    pub(crate) fn new(error: &Error) -> Self {
        Self {
            name: error.category().class_name(),
            code: error.code(),
            message: error.to_string(),
        }
    }

    /// The error object to hand over rather than throw.
    pub(crate) fn object<'env>(&self, env: &'env Env) -> napi::Result<Object<'env>> {
        object(env, "Error", self.name, self.code, &self.message)
    }
}

/// A core failure, as the error JavaScript sees.
pub(crate) fn to_js(env: &Env, error: &Error) -> napi::Error {
    let Parts {
        name,
        code,
        message,
    } = Parts::new(error);
    build(env, "Error", name, code, message)
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
