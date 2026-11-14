//! The raw segment channel, armed until it closes.

use std::sync::Arc;

use govee_toolkit::SegmentStream as CoreStream;
use napi::Env;
use napi::bindgen_prelude::{PromiseRaw, Uint8Array};
use napi_derive::napi;

use crate::conv::{Channels, colors, rgb};
use crate::errors::map;
use crate::promise::promise;

/// An open segment channel. The writers never block: each one replaces what
/// the next frame carries.
///
/// Close it, or leave it with `await using`. Garbage collection also disarms
/// the channel, and reports no failure. A write after `close()` throws
/// `stream_closed`.
#[napi]
pub struct SegmentStream {
    inner: Arc<CoreStream>,
}

impl SegmentStream {
    pub(crate) fn new(stream: CoreStream) -> Self {
        Self {
            inner: Arc::new(stream),
        }
    }
}

#[napi]
impl SegmentStream {
    /// How many zones the frames carry. The firmware reads the count off the
    /// frame, so it never changes while the stream is open.
    #[napi(getter)]
    pub fn zones(&self) -> u32 {
        u32::try_from(self.inner.zones()).unwrap_or(u32::MAX)
    }

    /// How fast frames go out, in hertz.
    #[napi(getter)]
    pub fn rate_hz(&self) -> f64 {
        self.inner.rate_hz()
    }

    /// How many frames reached the wire.
    #[napi(getter)]
    pub fn frames_sent(&self) -> i64 {
        i64::try_from(self.inner.frames_sent()).unwrap_or(i64::MAX)
    }

    /// How many frames a later write replaced before they left.
    #[napi(getter)]
    pub fn frames_superseded(&self) -> i64 {
        i64::try_from(self.inner.frames_superseded()).unwrap_or(i64::MAX)
    }

    /// What the emitting task failed with, if it failed. The stream stops
    /// sending, and the writers keep answering.
    #[napi(getter)]
    pub fn error(&self) -> Option<String> {
        self.inner.error().map(|e| e.to_string())
    }

    /// State every zone. The count must be the stream's own.
    ///
    /// A `Uint8Array` of three bytes for every zone crosses the binding once.
    /// An array of colors crosses it ten times per zone, which a frame loop
    /// pays on every frame.
    #[napi]
    pub fn set_all(
        &self,
        env: &Env,
        #[napi(ts_arg_type = "Array<[number, number, number]> | Uint8Array")] colors: Channels<'_>,
    ) -> napi::Result<()> {
        let values = self::colors(env, &colors)?;
        map(env, self.inner.set_all(&values))
    }

    /// State one zone, by its zero-based index.
    #[napi]
    pub fn set_zone(
        &self,
        env: &Env,
        index: u32,
        #[napi(ts_arg_type = "[number, number, number] | Uint8Array")] color: Channels<'_>,
    ) -> napi::Result<()> {
        let color = rgb(env, &color)?;
        map(env, self.inner.set_zone(index as usize, color))
    }

    /// Put one color in every zone.
    #[napi]
    pub fn fill(
        &self,
        env: &Env,
        #[napi(ts_arg_type = "[number, number, number] | Uint8Array")] color: Channels<'_>,
    ) -> napi::Result<()> {
        let color = rgb(env, &color)?;
        map(env, self.inner.fill(color))
    }

    /// Put black in every zone. The channel stays armed.
    #[napi]
    pub fn clear(&self, env: &Env) -> napi::Result<()> {
        map(env, self.inner.clear())
    }

    /// What the next frame carries, in the run `setAll` takes back. It is a
    /// copy: a write to it paints nothing.
    #[napi]
    pub fn buffer(&self) -> Uint8Array {
        Uint8Array::new(self.inner.buffer().as_flattened().to_vec())
    }

    /// Disarm the channel and wait for the last frame to leave.
    #[napi]
    pub fn close<'env>(&self, env: &'env Env) -> napi::Result<PromiseRaw<'env, ()>> {
        let stream = Arc::clone(&self.inner);
        promise(env, async move { stream.close().await })
    }

    #[napi(js_name = "toString")]
    pub fn to_js_string(&self) -> String {
        format!(
            "SegmentStream(zones={}, rateHz={})",
            self.zones(),
            self.rate_hz()
        )
    }
}
