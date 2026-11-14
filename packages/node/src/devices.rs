//! Several devices, and the verbs each member answers alone.

use std::future::Future;

use govee_toolkit::{
    Answer, Music, Outcome as CoreOutcome, OwnedDevices, Served as CoreServed, input,
};
use napi::Env;
use napi::bindgen_prelude::{Object, PromiseRaw, Unknown};
use napi_derive::napi;

use crate::conv;
use crate::device::DeviceHandle;
use crate::errors::Parts;
use crate::promise::promise;
use crate::types::{Device, Served};

/// What one member answered.
#[napi]
#[derive(Clone)]
pub struct Outcome {
    id: String,
    mode: Option<String>,
    served: Option<CoreServed>,
    error: Option<Parts>,
    record: serde_json::Value,
}

#[napi]
impl Outcome {
    /// The member.
    #[napi(getter)]
    pub fn id(&self) -> String {
        self.id.clone()
    }

    /// Whether the call on this member succeeded.
    #[napi(getter)]
    pub fn ok(&self) -> bool {
        self.error.is_none()
    }

    /// The mode that served the call. `null` where it failed.
    #[napi(getter)]
    pub fn mode(&self) -> Option<String> {
        self.mode.clone()
    }

    /// The command served. `null` on a failure and for `ensureKnown()`.
    #[napi(getter)]
    pub fn served(&self) -> Option<Served> {
        self.served.clone().map(Served::from)
    }

    /// What the call on one device throws. `null` where it succeeded.
    #[napi(getter, ts_return_type = "Error | null")]
    pub fn error<'env>(&self, env: &'env Env) -> napi::Result<Option<Object<'env>>> {
        self.error
            .as_ref()
            .map(|parts| parts.object(env))
            .transpose()
    }

    /// The record every surface prints: `id`, `ok`, `mode`, `command`, and
    /// `error` as `{ code, message }`.
    #[napi(
        js_name = "toJSON",
        ts_return_type = "{ id: string; ok: boolean; mode: string | null; command: string | null; error: { code: string; message: string } | null }"
    )]
    pub fn to_json(&self) -> serde_json::Value {
        self.record.clone()
    }

    #[napi(js_name = "toString")]
    pub fn to_js_string(&self) -> String {
        match &self.mode {
            Some(mode) => format!("Outcome(id='{}', mode='{mode}')", self.id),
            None => format!("Outcome(id='{}', ok=false)", self.id),
        }
    }
}

impl Outcome {
    pub(crate) fn new<T: Answer>(outcome: &CoreOutcome<T>) -> Self {
        Self {
            id: outcome.id.to_string(),
            mode: outcome.mode().map(|mode| mode.to_string()),
            served: outcome.served().cloned(),
            error: outcome.error().map(Parts::new),
            record: outcome.to_json(),
        }
    }

    pub(crate) fn all<T: Answer>(outcomes: &[CoreOutcome<T>]) -> Vec<Self> {
        outcomes.iter().map(Self::new).collect()
    }
}

/// The devices that `Govee.devices()` selects. Every verb on it runs on
/// every member at once, answers one `Outcome` per member in member order,
/// and rejects for nothing a member does.
#[napi]
pub struct Devices {
    pub(crate) inner: OwnedDevices,
}

#[napi]
impl Devices {
    /// One handle per member, in the order of every outcome list. Each one
    /// carries the mode that `Govee.devices()` pins.
    #[napi(getter)]
    pub fn members(&self) -> Vec<DeviceHandle> {
        self.inner
            .members()
            .into_iter()
            .map(|inner| DeviceHandle { inner })
            .collect()
    }

    /// The number of members.
    #[napi(getter)]
    pub fn length(&self) -> u32 {
        u32::try_from(self.inner.ids().len()).unwrap_or(u32::MAX)
    }

    /// What the SDK holds for each member: the SKU, the name, the groups, the
    /// modes and the health. Reads no hardware.
    ///
    /// A member that no transport knows and that the configuration pins no
    /// SKU for is not in the list. `members` holds every member.
    #[napi]
    pub fn list(&self) -> Vec<Device> {
        self.inner
            .devices()
            .list()
            .into_iter()
            .map(Device::from)
            .collect()
    }

    /// Scan for every member that no mode knows. Each outcome carries its mode.
    #[napi]
    pub fn ensure_known<'env>(
        &self,
        env: &'env Env,
    ) -> napi::Result<PromiseRaw<'env, Vec<Outcome>>> {
        let set = self.inner.clone();
        promise(env, async move {
            Ok(Outcome::all(&set.devices().ensure_known().await))
        })
    }

    /// Turn every member on or off.
    #[napi]
    pub fn power<'env>(
        &self,
        env: &'env Env,
        on: bool,
    ) -> napi::Result<PromiseRaw<'env, Vec<Outcome>>> {
        self.served(env, move |set| async move { set.devices().power(on).await })
    }

    /// Set the level on every member, against its own range.
    #[napi]
    pub fn brightness<'env>(
        &self,
        env: &'env Env,
        level: i64,
    ) -> napi::Result<PromiseRaw<'env, Vec<Outcome>>> {
        self.served(env, move |set| async move {
            set.devices().brightness(level).await
        })
    }

    /// Set one color on every member.
    #[napi]
    pub fn color<'env>(
        &self,
        env: &'env Env,
        #[napi(ts_arg_type = "[number, number, number] | Uint8Array")] rgb: conv::Channels<'_>,
    ) -> napi::Result<PromiseRaw<'env, Vec<Outcome>>> {
        let rgb = conv::rgb(env, &rgb)?;
        self.served(
            env,
            move |set| async move { set.devices().color(rgb).await },
        )
    }

    /// Set the white temperature on every member, in kelvin.
    #[napi]
    pub fn color_temp<'env>(
        &self,
        env: &'env Env,
        kelvin: i64,
    ) -> napi::Result<PromiseRaw<'env, Vec<Outcome>>> {
        self.served(env, move |set| async move {
            set.devices().color_temp(kelvin).await
        })
    }

    /// `DeviceHandle.music()` on every member.
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
    ) -> napi::Result<PromiseRaw<'env, Vec<Outcome>>> {
        let color = color.map(|color| conv::rgb(env, &color)).transpose()?;
        let music = Music::new(effect, sensitivity, soft, color);
        self.served(
            env,
            move |set| async move { set.devices().music(&music).await },
        )
    }

    /// `DeviceHandle.segment()` on every member, against its own zones.
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
    ) -> napi::Result<PromiseRaw<'env, Vec<Outcome>>> {
        let painting = conv::read(
            env,
            input::painting(
                conv::colors(env, &colors)?,
                &conv::optional(env, zones.as_ref())?,
                &conv::optional(env, resolution.as_ref())?,
                gradient,
            ),
        )?;
        self.served(env, move |set| async move {
            set.devices().segment(&painting.paint()).await
        })
    }

    /// Set the interpolation between zones on every member.
    #[napi]
    pub fn gradient<'env>(
        &self,
        env: &'env Env,
        on: bool,
    ) -> napi::Result<PromiseRaw<'env, Vec<Outcome>>> {
        self.served(
            env,
            move |set| async move { set.devices().gradient(on).await },
        )
    }

    #[napi(js_name = "toString")]
    pub fn to_js_string(&self) -> String {
        let ids: Vec<String> = self.inner.ids().iter().map(ToString::to_string).collect();
        format!("Devices(members={ids:?})")
    }
}

impl Devices {
    fn served<'env, Fut>(
        &self,
        env: &'env Env,
        verb: impl FnOnce(OwnedDevices) -> Fut,
    ) -> napi::Result<PromiseRaw<'env, Vec<Outcome>>>
    where
        Fut: Future<Output = Vec<CoreOutcome<CoreServed>>> + Send + 'static,
    {
        let call = verb(self.inner.clone());
        promise(env, async move { Ok(Outcome::all(&call.await)) })
    }
}
