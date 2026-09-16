//! One device, and everything a caller can ask of it.

use std::future::Future;

use govee_toolkit::{
    DeviceId, Error, Govee as CoreGovee, Music, Paint, Served as CoreServed, StreamOptions,
    WifiCredentials, describe,
};
use napi::Env;
use napi::bindgen_prelude::{Object, PromiseRaw, Unknown};
use napi_derive::napi;

use crate::conv::{self, to_js};
use crate::errors::map;
use crate::events::StatusStream;
use crate::promise::promise;
use crate::stream::SegmentStream;
use crate::types::{DeviceStatus, Health, Reply, Served};

/// A handle on one identity. It holds no state of its own: every answer comes
/// from the SDK it was made by.
#[napi]
pub struct DeviceHandle {
    pub(crate) govee: CoreGovee,
    pub(crate) id: DeviceId,
}

#[napi]
impl DeviceHandle {
    /// The MAC the device reports, uppercased.
    #[napi(getter)]
    pub fn id(&self) -> String {
        self.id.to_string()
    }

    /// The modes enabled for it, in preference order.
    #[napi(getter)]
    pub fn modes(&self) -> Vec<String> {
        self.govee
            .device(&self.id)
            .modes()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    /// Its health in one mode. `null` when the transport that serves that
    /// mode has never heard from it.
    #[napi]
    pub fn health(&self, env: &Env, mode: String) -> napi::Result<Option<Health>> {
        Ok(self
            .govee
            .device(&self.id)
            .health(conv::mode(env, &mode)?)
            .map(Health::from))
    }

    /// The mode a command sent now would go over. Read from recorded state,
    /// so the answer can change before the next call.
    #[napi]
    pub fn serving_mode(&self, env: &Env) -> napi::Result<String> {
        Ok(map(env, self.govee.device(&self.id).serving_mode())?.to_string())
    }

    /// What `devices/<SKU>.yaml` declares for it. Reads no hardware.
    #[napi]
    pub fn spec(&self, env: &Env) -> napi::Result<serde_json::Value> {
        to_js(env, map(env, self.govee.device(&self.id).spec())?)
    }

    /// What `devices/<SKU>.yaml` declares, as the record `govee describe`
    /// prints.
    #[napi]
    pub fn describe(&self, env: &Env) -> napi::Result<serde_json::Value> {
        to_js(
            env,
            &describe(map(env, self.govee.device(&self.id).spec())?),
        )
    }

    /// The last status heard, without asking for a new one.
    #[napi]
    pub fn last_status(&self) -> Option<DeviceStatus> {
        self.govee
            .device(&self.id)
            .last_status()
            .map(DeviceStatus::from)
    }

    /// Watch its status as answers arrive, over the mode that would serve a
    /// command now. `null` when no enabled mode can, or when that transport
    /// has heard nothing.
    #[napi]
    pub fn watch_status(&self) -> Option<StatusStream> {
        self.govee
            .device(&self.id)
            .watch_status()
            .map(StatusStream::new)
    }

    /// Scan for the device if no mode knows it yet, then answer the mode a
    /// command would go over.
    #[napi]
    pub fn ensure_known<'env>(&self, env: &'env Env) -> napi::Result<PromiseRaw<'env, String>> {
        let (govee, id) = self.parts();
        promise(env, async move {
            Ok(govee.ensure_known(&id).await?.to_string())
        })
    }

    /// Send a command, named as the device file names it.
    ///
    /// Each value is read under the type the entry declares for it. A value
    /// outside the declared range throws with the code the codec gives it,
    /// and nothing is sent.
    #[napi]
    pub fn send<'env>(
        &self,
        env: &'env Env,
        command: String,
        #[napi(
            ts_arg_type = "Record<string, boolean | number | string | Uint8Array | Array<number> | Array<[number, number, number]>>"
        )]
        args: Option<Object<'_>>,
    ) -> napi::Result<PromiseRaw<'env, Served>> {
        let supplied = conv::args(env, args)?;
        self.served(env, |govee, id| async move {
            let handle = govee.device(&id);
            let call = handle.resolve()?;
            let values = call.args(&command, supplied)?;
            call.send(&command, &values).await
        })
    }

    /// Run a command's exchanges and return what its `reply:` layouts
    /// captured.
    #[napi]
    pub fn read<'env>(
        &self,
        env: &'env Env,
        command: String,
        #[napi(
            ts_arg_type = "Record<string, boolean | number | string | Uint8Array | Array<number> | Array<[number, number, number]>>"
        )]
        args: Option<Object<'_>>,
    ) -> napi::Result<PromiseRaw<'env, Reply>> {
        let supplied = conv::args(env, args)?;
        let (govee, id) = self.parts();
        promise(env, async move {
            let handle = govee.device(&id);
            let call = handle.resolve()?;
            let values = call.args(&command, supplied)?;
            Ok(Reply::from(call.read(&command, &values).await?))
        })
    }

    /// Ask the device for its state and wait for the answer.
    #[napi]
    pub fn status<'env>(&self, env: &'env Env) -> napi::Result<PromiseRaw<'env, DeviceStatus>> {
        let (govee, id) = self.parts();
        promise(env, async move {
            Ok(DeviceStatus::from(govee.device(&id).status().await?))
        })
    }

    /// Turn the device on or off.
    #[napi]
    pub fn power<'env>(&self, env: &'env Env, on: bool) -> napi::Result<PromiseRaw<'env, Served>> {
        self.served(
            env,
            |govee, id| async move { govee.device(&id).power(on).await },
        )
    }

    /// Set the level, in the unit the device file declares. A level outside
    /// that range is an error, never a clamp.
    #[napi]
    pub fn brightness<'env>(
        &self,
        env: &'env Env,
        level: i64,
    ) -> napi::Result<PromiseRaw<'env, Served>> {
        self.served(env, |govee, id| async move {
            govee.device(&id).brightness(level).await
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
        self.served(env, |govee, id| async move {
            govee.device(&id).color(rgb).await
        })
    }

    /// Set the white temperature, in kelvin. It ends color mode.
    #[napi]
    pub fn color_temp<'env>(
        &self,
        env: &'env Env,
        kelvin: i64,
    ) -> napi::Result<PromiseRaw<'env, Served>> {
        self.served(env, |govee, id| async move {
            govee.device(&id).color_temp(kelvin).await
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
        let color = color.map(|value| conv::rgb(env, &value)).transpose()?;
        let default = Music::default();
        let music = Music {
            effect,
            sensitivity: sensitivity.unwrap_or(default.sensitivity),
            soft: soft.unwrap_or(default.soft),
            color,
        };
        self.served(env, |govee, id| async move {
            govee.device(&id).music(&music).await
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
        #[napi(ts_arg_type = "number | 'app' | 'native'")] resolution: Option<Unknown<'_>>,
        gradient: Option<bool>,
    ) -> napi::Result<PromiseRaw<'env, Served>> {
        let colors = conv::colors(env, &colors)?;
        let resolution = conv::resolution_or_default(env, resolution.as_ref())?;
        let gradient = gradient.unwrap_or(false);
        self.served(env, |govee, id| async move {
            let paint = Paint {
                zones: zones.as_deref(),
                colors: &colors,
                resolution,
                gradient,
            };
            govee.device(&id).segment(&paint).await
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
        self.served(env, |govee, id| async move {
            govee.device(&id).gradient(on).await
        })
    }

    /// Put the device on a Wi-Fi network over `ble`.
    ///
    /// The device must be in Bluetooth range and closed in the phone
    /// controller. The password travels in plaintext: anything in Bluetooth
    /// range during the transfer reads it. The network must be 2.4 GHz.
    ///
    /// Answers `"accepted"` where the device acknowledged the transfer, and
    /// `"sent"` where its device file declares no acknowledgement.
    #[napi]
    pub fn provision_wifi<'env>(
        &self,
        env: &'env Env,
        network: String,
        password: String,
        utc_offset_hours: Option<u8>,
        utc_offset_minutes: Option<u8>,
    ) -> napi::Result<PromiseRaw<'env, String>> {
        let credentials = WifiCredentials {
            network,
            password,
            utc_offset_hours: utc_offset_hours.unwrap_or(0),
            utc_offset_minutes: utc_offset_minutes.unwrap_or(0),
        };
        let (govee, id) = self.parts();
        promise(env, async move {
            let done = govee.device(&id).provision_wifi(&credentials).await?;
            Ok(done.as_str().to_owned())
        })
    }

    /// Open the raw segment channel and paint it frame by frame.
    ///
    /// Power the device on first: arming a dark strip paints nothing. The
    /// channel holds the colors only while it is armed, and the device goes
    /// back to the color it showed before once the stream closes.
    ///
    /// `resolution` takes `"app"` when it is `null`, and `rate` takes
    /// `"measured"`.
    #[napi]
    pub fn open_stream<'env>(
        &self,
        env: &'env Env,
        #[napi(ts_arg_type = "number | 'app' | 'native'")] resolution: Option<Unknown<'_>>,
        #[napi(ts_arg_type = "number | 'measured'")] rate: Option<Unknown<'_>>,
        gradient: Option<bool>,
    ) -> napi::Result<PromiseRaw<'env, SegmentStream>> {
        let options = StreamOptions {
            resolution: conv::resolution_or_default(env, resolution.as_ref())?,
            rate: conv::rate_or_default(env, rate.as_ref())?,
            gradient: gradient.unwrap_or(false),
        };
        let (govee, id) = self.parts();
        promise(env, async move {
            Ok(SegmentStream::new(
                govee.device(&id).open_stream(options).await?,
            ))
        })
    }

    #[napi(js_name = "toString")]
    pub fn to_js_string(&self) -> String {
        format!("DeviceHandle(id='{}')", self.id)
    }
}

impl DeviceHandle {
    fn parts(&self) -> (CoreGovee, DeviceId) {
        (self.govee.clone(), self.id.clone())
    }

    fn served<'env, Fut>(
        &self,
        env: &'env Env,
        verb: impl FnOnce(CoreGovee, DeviceId) -> Fut,
    ) -> napi::Result<PromiseRaw<'env, Served>>
    where
        Fut: Future<Output = Result<CoreServed, Error>> + Send + 'static,
    {
        let (govee, id) = self.parts();
        let call = verb(govee, id);
        promise(env, async move { Ok(Served::from(call.await?)) })
    }
}
