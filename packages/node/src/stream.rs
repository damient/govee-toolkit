//! The raw segment channel, armed until it closes.

use std::sync::{Mutex, MutexGuard};

use govee_toolkit::SegmentStream as CoreStream;
use napi::Env;
use napi::bindgen_prelude::{PromiseRaw, Unknown};
use napi_derive::napi;

use crate::conv::{colors, rgb};
use crate::errors::{map, value_error};
use crate::promise::promise;

/// An open segment channel. The writers never block: each one replaces what
/// the next frame carries.
///
/// Close it, or leave it with `await using`. A stream that is collected
/// disarms the channel as well, and reports no failure.
#[napi]
pub struct SegmentStream {
    /// Taken by `close`, which awaits the disarm.
    inner: Mutex<Option<CoreStream>>,
    zones: u32,
    rate_hz: f64,
}

impl SegmentStream {
    pub(crate) fn new(stream: CoreStream) -> Self {
        let zones = u32::try_from(stream.zones()).unwrap_or(u32::MAX);
        let rate_hz = stream.rate_hz();
        Self {
            inner: Mutex::new(Some(stream)),
            zones,
            rate_hz,
        }
    }

    /// The open channel, or what a writer that panicked left behind.
    fn locked(&self, env: &Env) -> napi::Result<MutexGuard<'_, Option<CoreStream>>> {
        self.inner
            .lock()
            .map_err(|_| value_error(env, "this stream failed while another call held it"))
    }

    fn with<T>(
        &self,
        env: &Env,
        call: impl FnOnce(&CoreStream) -> Result<T, govee_toolkit::Error>,
    ) -> napi::Result<T> {
        let guard = self.locked(env)?;
        let stream = guard
            .as_ref()
            .ok_or_else(|| value_error(env, "this stream is closed"))?;
        map(env, call(stream))
    }
}

#[napi]
impl SegmentStream {
    /// How many zones the frames carry. The firmware reads the count off the
    /// frame, so it never changes while the stream is open.
    #[napi(getter)]
    pub fn zones(&self) -> u32 {
        self.zones
    }

    /// How fast frames go out, in hertz.
    #[napi(getter)]
    pub fn rate_hz(&self) -> f64 {
        self.rate_hz
    }

    /// How many frames reached the wire.
    #[napi(getter)]
    pub fn frames_sent(&self, env: &Env) -> napi::Result<i64> {
        self.with(env, |stream| {
            Ok(i64::try_from(stream.frames_sent()).unwrap_or(i64::MAX))
        })
    }

    /// How many frames a later write replaced before they left.
    #[napi(getter)]
    pub fn frames_superseded(&self, env: &Env) -> napi::Result<i64> {
        self.with(env, |stream| {
            Ok(i64::try_from(stream.frames_superseded()).unwrap_or(i64::MAX))
        })
    }

    /// What the emitting task failed with, if it failed. The stream stops
    /// sending, and the writers keep answering.
    #[napi(getter)]
    pub fn error(&self, env: &Env) -> napi::Result<Option<String>> {
        self.with(env, |stream| Ok(stream.error().map(|e| e.to_string())))
    }

    /// State every zone. The count must be the stream's own.
    #[napi]
    pub fn set_all(
        &self,
        env: &Env,
        #[napi(ts_arg_type = "Array<[number, number, number]>")] colors: Unknown<'_>,
    ) -> napi::Result<()> {
        let values = self::colors(env, &colors)?;
        self.with(env, |stream| stream.set_all(&values))
    }

    /// State one zone, by its zero-based index.
    #[napi]
    pub fn set_zone(
        &self,
        env: &Env,
        index: u32,
        #[napi(ts_arg_type = "[number, number, number]")] color: Unknown<'_>,
    ) -> napi::Result<()> {
        let color = rgb(env, &color)?;
        self.with(env, |stream| stream.set_zone(index as usize, color))
    }

    /// Put one color in every zone.
    #[napi]
    pub fn fill(
        &self,
        env: &Env,
        #[napi(ts_arg_type = "[number, number, number]")] color: Unknown<'_>,
    ) -> napi::Result<()> {
        let color = rgb(env, &color)?;
        self.with(env, |stream| stream.fill(color))
    }

    /// Put black in every zone. The channel stays armed.
    #[napi]
    pub fn clear(&self, env: &Env) -> napi::Result<()> {
        self.with(env, CoreStream::clear)
    }

    /// What the next frame carries.
    #[napi(ts_return_type = "Array<[number, number, number]>")]
    pub fn buffer(&self, env: &Env) -> napi::Result<Vec<Vec<u8>>> {
        self.with(env, |stream| Ok(stream.buffer()))
            .map(|colors| colors.into_iter().map(|color| color.to_vec()).collect())
    }

    /// Disarm the channel and wait for the last frame to leave.
    #[napi]
    pub fn close<'env>(&self, env: &'env Env) -> napi::Result<PromiseRaw<'env, ()>> {
        let taken = self.locked(env)?.take();
        promise(env, async move {
            if let Some(stream) = taken {
                stream.close().await?;
            }
            Ok(())
        })
    }

    #[napi(js_name = "toString")]
    pub fn to_js_string(&self) -> String {
        format!(
            "SegmentStream(zones={}, rateHz={})",
            self.zones, self.rate_hz
        )
    }
}
