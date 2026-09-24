//! The verbs a person names, on one device.
//!
//! Each one calls the matching method of the crate, which reads the entry the
//! device file marks with that `role:`. No command name lives here.

use govee_toolkit::{Identify, Paint};
use napi::Env;
use napi::bindgen_prelude::{PromiseRaw, Unknown};
use napi_derive::napi;

use crate::conv;
use crate::device::DeviceHandle;
use crate::promise::promise;
use crate::types::Served;

#[napi]
impl DeviceHandle {
    /// Turn the device on or off.
    #[napi]
    pub fn power<'env>(&self, env: &'env Env, on: bool) -> napi::Result<PromiseRaw<'env, Served>> {
        self.served(env, |govee, pinned, id| async move {
            govee.device_maybe_on(&id, pinned).power(on).await
        })
    }

    /// Set the level, in the unit the device file declares. A level outside
    /// that range is an error, never a clamp.
    #[napi]
    pub fn brightness<'env>(
        &self,
        env: &'env Env,
        level: i64,
    ) -> napi::Result<PromiseRaw<'env, Served>> {
        self.served(env, |govee, pinned, id| async move {
            govee.device_maybe_on(&id, pinned).brightness(level).await
        })
    }

    /// Set one color, as three channels or as three bytes.
    #[napi]
    pub fn color<'env>(
        &self,
        env: &'env Env,
        #[napi(ts_arg_type = "[number, number, number] | Uint8Array")] rgb: conv::Channels<'_>,
    ) -> napi::Result<PromiseRaw<'env, Served>> {
        let rgb = conv::rgb(env, &rgb)?;
        self.served(env, |govee, pinned, id| async move {
            govee.device_maybe_on(&id, pinned).color(rgb).await
        })
    }

    /// Power the device on and paint one color, so a person sees which
    /// fixture this identity drives.
    ///
    /// The look the device held is lost. To walk a rig, power every device
    /// off, wait a second, and then call this on one device at a time.
    ///
    /// `null` takes the core's defaults: green, and the top of the
    /// brightness range the device file declares.
    #[napi]
    pub fn identify<'env>(
        &self,
        env: &'env Env,
        #[napi(ts_arg_type = "[number, number, number] | Uint8Array")] color: Option<
            conv::Channels<'_>,
        >,
        full_brightness: Option<bool>,
    ) -> napi::Result<PromiseRaw<'env, ()>> {
        let default = Identify::default();
        let options = Identify {
            color: match color {
                Some(rgb) => conv::rgb(env, &rgb)?,
                None => default.color,
            },
            full_brightness: full_brightness.unwrap_or(default.full_brightness),
        };
        let (govee, pinned, id) = self.parts();
        promise(env, async move {
            govee
                .device_maybe_on(&id, pinned)
                .identify(&options)
                .await?;
            Ok(())
        })
    }

    /// Set the white temperature, in kelvin. It ends color mode.
    #[napi]
    pub fn color_temp<'env>(
        &self,
        env: &'env Env,
        kelvin: i64,
    ) -> napi::Result<PromiseRaw<'env, Served>> {
        self.served(env, |govee, pinned, id| async move {
            govee.device_maybe_on(&id, pinned).color_temp(kelvin).await
        })
    }

    /// Play an effect the device renders from its own microphone.
    ///
    /// The identifiers are the mode's own: one the entry accepts is not one
    /// the device renders. `color` imposes a color, and `null` leaves the
    /// colors to the firmware.
    ///
    /// `null` takes the core's default for `sensitivity` and for `soft`.
    #[napi]
    pub fn music<'env>(
        &self,
        env: &'env Env,
        effect: i64,
        sensitivity: Option<i64>,
        soft: Option<bool>,
        #[napi(ts_arg_type = "[number, number, number] | Uint8Array")] color: Option<
            conv::Channels<'_>,
        >,
    ) -> napi::Result<PromiseRaw<'env, Served>> {
        let music = conv::music(env, effect, sensitivity, soft, color.as_ref())?;
        self.served(env, |govee, pinned, id| async move {
            govee.device_maybe_on(&id, pinned).music(&music).await
        })
    }

    /// Paint the segments once.
    ///
    /// One color fills every zone. An array of colors, or a `Uint8Array` of
    /// three bytes per zone, states them all. A zone list takes one color.
    /// `resolution` takes `"app"` when it is `null`.
    #[napi]
    pub fn segment<'env>(
        &self,
        env: &'env Env,
        #[napi(
            ts_arg_type = "[number, number, number] | Array<[number, number, number]> | Uint8Array"
        )]
        colors: conv::Channels<'_>,
        zones: Option<Vec<u16>>,
        #[napi(ts_arg_type = "number | 'app' | 'native' | 'groups'")] resolution: Option<
            Unknown<'_>,
        >,
        gradient: Option<bool>,
    ) -> napi::Result<PromiseRaw<'env, Served>> {
        let colors = conv::colors(env, &colors)?;
        let resolution = conv::resolution_or_default(env, resolution.as_ref())?;
        let gradient = gradient.unwrap_or(false);
        self.served(env, |govee, pinned, id| async move {
            let paint = Paint {
                zones: zones.as_deref(),
                colors: &colors,
                resolution,
                gradient,
            };
            govee.device_maybe_on(&id, pinned).segment(&paint).await
        })
    }

    /// Ask the firmware to interpolate between zones, and to wrap from the
    /// last zone back to the first.
    #[napi]
    pub fn gradient<'env>(
        &self,
        env: &'env Env,
        on: bool,
    ) -> napi::Result<PromiseRaw<'env, Served>> {
        self.served(env, |govee, pinned, id| async move {
            govee.device_maybe_on(&id, pinned).gradient(on).await
        })
    }
}
