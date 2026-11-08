//! Several devices, and the verbs each member answers alone.

use std::future::Future;

use govee_toolkit::codec::Mode;
use govee_toolkit::{
    DeviceId, Devices as CoreDevices, Error, Govee, Outcome as CoreOutcome, Paint,
    Served as CoreServed,
};
use napi::Env;
use napi::bindgen_prelude::{Object, PromiseRaw, Unknown};
use napi_derive::napi;

use crate::conv;
use crate::device::DeviceHandle;
use crate::errors::{Parts, map};
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

    #[napi(js_name = "toString")]
    pub fn to_js_string(&self) -> String {
        match &self.mode {
            Some(mode) => format!("Outcome(id='{}', mode='{mode}')", self.id),
            None => format!("Outcome(id='{}', ok=false)", self.id),
        }
    }
}

impl Outcome {
    pub(crate) fn new<T>(
        outcome: CoreOutcome<T>,
        read: impl FnOnce(T) -> (Mode, Option<CoreServed>),
    ) -> Self {
        let id = outcome.id.to_string();
        match outcome.result {
            Ok(value) => {
                let (mode, served) = read(value);
                Self {
                    id,
                    mode: Some(mode.to_string()),
                    served,
                    error: None,
                }
            }
            Err(error) => Self {
                id,
                mode: None,
                served: None,
                error: Some(Parts::new(&error)),
            },
        }
    }
}

/// The devices that `Govee.devices()` selects. Every verb on it runs on
/// every member at once, answers one `Outcome` per member in member order,
/// and rejects for nothing a member does.
#[napi]
pub struct Devices {
    pub(crate) govee: Govee,
    pub(crate) pinned: Option<Mode>,
    pub(crate) ids: Vec<DeviceId>,
}

#[napi]
impl Devices {
    /// One handle per member, in the order of every outcome list. Each one
    /// carries the mode that `Govee.devices()` pins.
    #[napi(getter)]
    pub fn members(&self) -> Vec<DeviceHandle> {
        self.ids
            .iter()
            .map(|id| DeviceHandle {
                govee: self.govee.clone(),
                pinned: self.pinned,
                id: id.clone(),
            })
            .collect()
    }

    /// The number of members.
    #[napi(getter)]
    pub fn length(&self) -> u32 {
        u32::try_from(self.ids.len()).unwrap_or(u32::MAX)
    }

    /// What the SDK holds for each member: the SKU, the name, the groups, the
    /// modes and the health. Reads no hardware.
    ///
    /// A member that no transport knows and that the configuration pins no
    /// SKU for is not in the list. `members` holds every member.
    #[napi]
    pub fn list(&self, env: &Env) -> napi::Result<Vec<Device>> {
        Ok(map(env, devices(&self.govee, &self.ids, self.pinned))?
            .list()
            .into_iter()
            .map(Device::from)
            .collect())
    }

    /// Scan for every member that no mode knows. Each outcome carries its mode.
    #[napi]
    pub fn ensure_known<'env>(
        &self,
        env: &'env Env,
    ) -> napi::Result<PromiseRaw<'env, Vec<Outcome>>> {
        let (govee, pinned, ids) = self.parts();
        promise(env, async move {
            let known = devices(&govee, &ids, pinned)?.ensure_known().await;
            Ok(known
                .into_iter()
                .map(|outcome| Outcome::new(outcome, |mode| (mode, None)))
                .collect())
        })
    }

    /// Turn every member on or off.
    #[napi]
    pub fn power<'env>(
        &self,
        env: &'env Env,
        on: bool,
    ) -> napi::Result<PromiseRaw<'env, Vec<Outcome>>> {
        self.served(env, move |govee, pinned, ids| async move {
            Ok(devices(&govee, &ids, pinned)?.power(on).await)
        })
    }

    /// Set the level on every member, against its own range.
    #[napi]
    pub fn brightness<'env>(
        &self,
        env: &'env Env,
        level: i64,
    ) -> napi::Result<PromiseRaw<'env, Vec<Outcome>>> {
        self.served(env, move |govee, pinned, ids| async move {
            Ok(devices(&govee, &ids, pinned)?.brightness(level).await)
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
        self.served(env, move |govee, pinned, ids| async move {
            Ok(devices(&govee, &ids, pinned)?.color(rgb).await)
        })
    }

    /// Set the white temperature on every member, in kelvin.
    #[napi]
    pub fn color_temp<'env>(
        &self,
        env: &'env Env,
        kelvin: i64,
    ) -> napi::Result<PromiseRaw<'env, Vec<Outcome>>> {
        self.served(env, move |govee, pinned, ids| async move {
            Ok(devices(&govee, &ids, pinned)?.color_temp(kelvin).await)
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
        let music = conv::music(env, effect, sensitivity, soft, color.as_ref())?;
        self.served(env, move |govee, pinned, ids| async move {
            Ok(devices(&govee, &ids, pinned)?.music(&music).await)
        })
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
        zones: Option<Vec<u16>>,
        #[napi(ts_arg_type = "number | 'app' | 'native' | 'groups'")] resolution: Option<
            Unknown<'_>,
        >,
        gradient: Option<bool>,
    ) -> napi::Result<PromiseRaw<'env, Vec<Outcome>>> {
        let colors = conv::colors(env, &colors)?;
        let resolution = conv::resolution_or_default(env, resolution.as_ref())?;
        let gradient = gradient.unwrap_or(false);
        self.served(env, move |govee, pinned, ids| async move {
            let paint = Paint {
                zones: zones.as_deref(),
                colors: &colors,
                resolution,
                gradient,
            };
            Ok(devices(&govee, &ids, pinned)?.segment(&paint).await)
        })
    }

    /// Set the interpolation between zones on every member.
    #[napi]
    pub fn gradient<'env>(
        &self,
        env: &'env Env,
        on: bool,
    ) -> napi::Result<PromiseRaw<'env, Vec<Outcome>>> {
        self.served(env, move |govee, pinned, ids| async move {
            Ok(devices(&govee, &ids, pinned)?.gradient(on).await)
        })
    }

    #[napi(js_name = "toString")]
    pub fn to_js_string(&self) -> String {
        let ids: Vec<String> = self.ids.iter().map(ToString::to_string).collect();
        format!("Devices(members={ids:?})")
    }
}

/// The core handle over `ids`. It parses nothing and scans nothing.
/// The core set of the stored identities. It parses nothing and scans
/// nothing.
pub(crate) fn devices<'a>(
    govee: &'a Govee,
    ids: &[DeviceId],
    pinned: Option<Mode>,
) -> Result<CoreDevices<'a>, Error> {
    ids.iter().map(|id| govee.device(id, pinned)).collect()
}

impl Devices {
    pub(crate) fn parts(&self) -> (Govee, Option<Mode>, Vec<DeviceId>) {
        (self.govee.clone(), self.pinned, self.ids.clone())
    }

    fn served<'env, Fut>(
        &self,
        env: &'env Env,
        verb: impl FnOnce(Govee, Option<Mode>, Vec<DeviceId>) -> Fut,
    ) -> napi::Result<PromiseRaw<'env, Vec<Outcome>>>
    where
        Fut: Future<Output = Result<Vec<CoreOutcome<CoreServed>>, Error>> + Send + 'static,
    {
        let (govee, pinned, ids) = self.parts();
        let call = verb(govee, pinned, ids);
        promise(env, async move {
            Ok(call
                .await?
                .into_iter()
                .map(|outcome| Outcome::new(outcome, |served| (served.mode, Some(served))))
                .collect())
        })
    }
}
