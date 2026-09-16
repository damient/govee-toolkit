//! How an asynchronous core call reaches JavaScript.

use std::future::Future;

use govee_toolkit::Error;
use napi::Env;
use napi::bindgen_prelude::{PromiseRaw, ToNapiValue};

use crate::errors::to_js;

/// Drive one core call and answer the promise it settles.
///
/// The failure crosses on the JavaScript thread, which is the only place the
/// error object can be built.
pub(crate) fn promise<T, Fut>(env: &Env, call: Fut) -> napi::Result<PromiseRaw<'_, T>>
where
    T: ToNapiValue + Send + 'static,
    Fut: Future<Output = Result<T, Error>> + Send + 'static,
{
    env.spawn_future_with_callback(async move { Ok(call.await) }, |env, answer| {
        answer.map_err(|error| to_js(env, &error))
    })
}
