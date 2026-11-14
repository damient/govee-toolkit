//! The verbs a person names, on one device.
//!
//! Each one calls the matching method of the crate, which reads the entry the
//! device file marks with that `role:`. No command name lives here.

use govee_toolkit::{Identify, Music, input};
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
        self.served(env, |owned| async move { owned.handle().power(on).await })
    }

    /// Set the level, in the unit the device file declares. A level outside
    /// that range is an error, never a clamp.
    #[napi]
    pub fn brightness<'env>(
        &self,
        env: &'env Env,
        level: i64,
    ) -> napi::Result<PromiseRaw<'env, Served>> {
        self.served(env, |owned| async move {
            owned.handle().brightness(level).await
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
        self.served(env, |owned| async move { owned.handle().color(rgb).await })
    }

    /// Power the device on and paint one color, so a person sees which
    /// fixture this identity drives.
    ///
    /// One pass: the device stays on and lit, and loses the look it held.
    /// `Govee.identify()` runs the whole walk.
    ///
    /// `fullBrightness` sets the top of the declared brightness range.
    ///
    /// @param [color=[0, 255, 0]]
    /// @param [fullBrightness=true]
    #[napi]
    pub fn identify<'env>(
        &self,
        env: &'env Env,
        #[napi(ts_arg_type = "[number, number, number] | Uint8Array")] color: Option<
            conv::Channels<'_>,
        >,
        full_brightness: Option<bool>,
    ) -> napi::Result<PromiseRaw<'env, ()>> {
        let color = color.map(|color| conv::rgb(env, &color)).transpose()?;
        let options = Identify::new(color, full_brightness);
        let owned = self.inner.clone();
        promise(env, async move {
            owned.handle().identify(&options).await?;
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
        self.served(env, |owned| async move {
            owned.handle().color_temp(kelvin).await
        })
    }

    /// Play an effect the device renders from its own microphone.
    ///
    /// The identifiers are the mode's own: one the entry accepts is not one
    /// the device renders. `color` imposes a color, and `null` leaves the
    /// colors to the firmware.
    ///
    /// @param [sensitivity=50]
    /// @param [soft=false]
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
        let color = color.map(|color| conv::rgb(env, &color)).transpose()?;
        let music = Music::new(effect, sensitivity, soft, color);
        self.served(
            env,
            |owned| async move { owned.handle().music(&music).await },
        )
    }

    /// Paint the segments once.
    ///
    /// One color fills every zone. An array of colors, or a `Uint8Array` of
    /// three bytes per zone, states them all. A zone list takes one color.
    ///
    /// @param [resolution='app']
    /// @param [gradient=false]
    #[napi]
    pub fn segment<'env>(
        &self,
        env: &'env Env,
        #[napi(
            ts_arg_type = "[number, number, number] | Array<[number, number, number]> | Uint8Array"
        )]
        colors: conv::Channels<'_>,
        #[napi(ts_arg_type = "Array<number> | undefined | null")] zones: Option<Unknown<'_>>,
        #[napi(ts_arg_type = "number | 'app' | 'native' | 'groups'")] resolution: Option<
            Unknown<'_>,
        >,
        gradient: Option<bool>,
    ) -> napi::Result<PromiseRaw<'env, Served>> {
        let painting = conv::read(
            env,
            input::painting(
                conv::colors(env, &colors)?,
                &conv::optional(env, zones.as_ref())?,
                &conv::optional(env, resolution.as_ref())?,
                gradient,
            ),
        )?;
        self.served(env, move |owned| async move {
            owned.handle().segment(&painting.paint()).await
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
        self.served(
            env,
            |owned| async move { owned.handle().gradient(on).await },
        )
    }
}
