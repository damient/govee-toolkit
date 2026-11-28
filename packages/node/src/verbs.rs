//! The verbs a person names, on one device.
//!
//! Each one calls the matching method of the crate, which reads the entry the
//! device file marks with that `role:`. No command name lives here.

use govee_toolkit::codec::Role;
use govee_toolkit::input::Value;
use govee_toolkit::{Drawing, Identify, Music, input};
use napi::Env;
use napi::bindgen_prelude::{Object, PromiseRaw, Unknown};
use napi_derive::napi;

use crate::conv;
use crate::device::DeviceHandle;
use crate::errors::value_error;
use crate::promise::promise;
use crate::scenes::{self, Query};
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

    /// Paint the segments once, through the zone mask of the device file.
    ///
    /// One color fills every zone. An array of colors, or a `Uint8Array` of
    /// three bytes per zone, states them all. A zone list takes one color.
    /// The paint arms no channel, and the colors stay. The `'native'`
    /// resolution throws `native_zones_unreachable`: stream instead.
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

    /// Play a scene. A name or a code reads the library that
    /// `fetchScenes()` stored for the model of the device. A `Scene` goes
    /// out as it is.
    ///
    /// Over `lan` the device answers nothing: a scene that it does not play
    /// looks like one that it plays.
    #[napi]
    pub fn scene<'env>(
        &self,
        env: &'env Env,
        #[napi(ts_arg_type = "string | number | Scene")] scene: Unknown<'_>,
    ) -> napi::Result<PromiseRaw<'env, Served>> {
        match scenes::query(env, &scene)? {
            Query::Named(query) => self.served(env, |owned| async move {
                owned.handle().scene_named(&query).await
            }),
            Query::Scene(scene) => {
                self.served(
                    env,
                    |owned| async move { owned.handle().scene(&scene).await },
                )
            }
        }
    }

    /// Store a drawing of the LEDs. The device keeps playing it after the
    /// sender stops.
    ///
    /// `cells` names the LEDs, zero-based, and the other LEDs show the
    /// background. `colors` holds one color, or one color per cell. Without
    /// `cells`, it holds one color per zone of `resolution`. `args` sets the
    /// other arguments of the drawing entry by name, such as `speed`. The
    /// resolution defaults to `'app'`.
    #[napi]
    pub fn draw<'env>(
        &self,
        env: &'env Env,
        #[napi(
            ts_arg_type = "{ cells?: Array<number>, colors: [number, number, number] | Array<[number, number, number]> | Uint8Array, resolution?: number | 'app' | 'native' | 'groups', args?: Record<string, boolean | number | string | Uint8Array | Array<number> | Array<[number, number, number]>> }"
        )]
        drawing: Object<'_>,
    ) -> napi::Result<PromiseRaw<'env, Served>> {
        let value = conv::options(env, Some(&drawing))?;
        conv::read(env, value.known_keys("drawing", &DRAWING))?;
        let at = |key: &str| value.get(key).unwrap_or(&Value::Null);
        let Some(colors) = value.get("colors") else {
            return Err(value_error(env, "a drawing carries `colors`"));
        };
        let colors = conv::read(env, input::colors(colors, conv::STYLE))?;
        let cells = value
            .get("cells")
            .map(|cells| conv::read(env, input::zones(cells)))
            .transpose()?;
        let resolution = conv::read(env, input::resolution(at("resolution")))?;
        let supplied = conv::read(env, input::args(at("args"), conv::STYLE))?;
        self.served(env, move |owned| async move {
            let device = owned.handle();
            // The mode that serves the device now gives the types.
            let args = device.resolve()?.role_args(Role::Draw, supplied)?;
            let drawing = Drawing {
                cells: cells.as_deref(),
                colors: &colors,
                resolution,
                args: &args,
            };
            device.draw(&drawing).await
        })
    }
}

const DRAWING: [&str; 4] = ["cells", "colors", "resolution", "args"];
