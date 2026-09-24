//! Several devices driven as one, and what each member answered.
//!
//! A verb on a group rejects for nothing a member does: every member answers
//! its own `Outcome`, and a member that fails stops no other one.

use std::future::Future;

use govee_toolkit::codec::Mode;
use govee_toolkit::{DeviceId, Govee, Outcome as CoreOutcome, Paint, Served as CoreServed};
use napi::Env;
use napi::bindgen_prelude::{Object, PromiseRaw, Unknown};
use napi_derive::napi;

use crate::conv;
use crate::errors::Parts;
use crate::types::Served;

/// What one member answered: `served` or `mode` where the call succeeded,
/// and `error` where it failed.
#[napi]
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

    /// The command that was served. `null` where the call failed, and for
    /// `ensureKnown()`.
    #[napi(getter)]
    pub fn served(&self) -> Option<Served> {
        self.served.clone().map(Served::from)
    }

    /// The error the call on one device would throw. `null` where it
    /// succeeded.
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
    fn new<T>(outcome: CoreOutcome<T>, read: impl FnOnce(T) -> (Mode, Option<CoreServed>)) -> Self {
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

/// A handle on the members of a group. It holds no state of its own.
#[napi]
pub struct GroupHandle {
    pub(crate) govee: Govee,
    pub(crate) pinned: Option<Mode>,
    pub(crate) members: Vec<DeviceId>,
}

#[napi]
impl GroupHandle {
    /// The identities of the members, in the order every outcome list
    /// follows.
    #[napi(getter)]
    pub fn members(&self) -> Vec<String> {
        self.members.iter().map(ToString::to_string).collect()
    }

    /// Scan for every member that no mode knows yet. Each outcome carries the
    /// mode a command would go over.
    #[napi]
    pub fn ensure_known<'env>(
        &self,
        env: &'env Env,
    ) -> napi::Result<PromiseRaw<'env, Vec<Outcome>>> {
        let (govee, pinned, members) = self.parts();
        env.spawn_future(async move {
            let known = govee.group_maybe_on(&members, pinned).ensure_known().await;
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
        self.served(env, move |govee, pinned, members| async move {
            govee.group_maybe_on(&members, pinned).power(on).await
        })
    }

    /// Set the level on every member. The range is each member's own.
    #[napi]
    pub fn brightness<'env>(
        &self,
        env: &'env Env,
        level: i64,
    ) -> napi::Result<PromiseRaw<'env, Vec<Outcome>>> {
        self.served(env, move |govee, pinned, members| async move {
            govee
                .group_maybe_on(&members, pinned)
                .brightness(level)
                .await
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
        self.served(env, move |govee, pinned, members| async move {
            govee.group_maybe_on(&members, pinned).color(rgb).await
        })
    }

    /// Set the white temperature on every member, in kelvin.
    #[napi]
    pub fn color_temp<'env>(
        &self,
        env: &'env Env,
        kelvin: i64,
    ) -> napi::Result<PromiseRaw<'env, Vec<Outcome>>> {
        self.served(env, move |govee, pinned, members| async move {
            govee
                .group_maybe_on(&members, pinned)
                .color_temp(kelvin)
                .await
        })
    }

    /// Play an effect on every member, as `DeviceHandle.music()` does.
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
        self.served(env, move |govee, pinned, members| async move {
            govee.group_maybe_on(&members, pinned).music(&music).await
        })
    }

    /// Paint the segments of every member once, as `DeviceHandle.segment()`
    /// does. A zone list reads against each member's own zones.
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
        self.served(env, move |govee, pinned, members| async move {
            let paint = Paint {
                zones: zones.as_deref(),
                colors: &colors,
                resolution,
                gradient,
            };
            govee.group_maybe_on(&members, pinned).segment(&paint).await
        })
    }

    /// Set the interpolation between zones on every member.
    #[napi]
    pub fn gradient<'env>(
        &self,
        env: &'env Env,
        on: bool,
    ) -> napi::Result<PromiseRaw<'env, Vec<Outcome>>> {
        self.served(env, move |govee, pinned, members| async move {
            govee.group_maybe_on(&members, pinned).gradient(on).await
        })
    }

    #[napi(js_name = "toString")]
    pub fn to_js_string(&self) -> String {
        format!("GroupHandle(members={:?})", self.members())
    }
}

impl GroupHandle {
    fn parts(&self) -> (Govee, Option<Mode>, Vec<DeviceId>) {
        (self.govee.clone(), self.pinned, self.members.clone())
    }

    fn served<'env, Fut>(
        &self,
        env: &'env Env,
        verb: impl FnOnce(Govee, Option<Mode>, Vec<DeviceId>) -> Fut,
    ) -> napi::Result<PromiseRaw<'env, Vec<Outcome>>>
    where
        Fut: Future<Output = Vec<CoreOutcome<CoreServed>>> + Send + 'static,
    {
        let (govee, pinned, members) = self.parts();
        let call = verb(govee, pinned, members);
        env.spawn_future(async move {
            Ok(call
                .await
                .into_iter()
                .map(|outcome| Outcome::new(outcome, |served| (served.mode, Some(served))))
                .collect())
        })
    }
}
