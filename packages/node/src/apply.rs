//! `DeviceHandle.invoke()` and `Devices.apply()`.

use govee_toolkit::{Applied as CoreApplied, Invoked as CoreInvoked, Secrets, input};
use napi::Env;
use napi::bindgen_prelude::{Object, PromiseRaw};
use napi_derive::napi;

use crate::conv::{self, STYLE};
use crate::device::DeviceHandle;
use crate::devices::{Devices, Outcome};
use crate::promise::promise;

/// What `DeviceHandle.invoke()` did with a command.
#[napi]
pub struct Invoked {
    id: String,
    mode: String,
    command: String,
    fields: Option<serde_json::Value>,
}

#[napi]
impl Invoked {
    /// The device.
    #[napi(getter)]
    pub fn id(&self) -> String {
        self.id.clone()
    }

    /// The mode that served the command.
    #[napi(getter)]
    pub fn mode(&self) -> String {
        self.mode.clone()
    }

    /// The command, as the device file names it.
    #[napi(getter)]
    pub fn command(&self) -> String {
        self.command.clone()
    }

    /// What the `reply:` layouts captured, or `null` for a command that was
    /// sent and not read.
    #[napi(getter)]
    pub fn fields(&self) -> Option<serde_json::Value> {
        self.fields.clone()
    }
}

#[napi]
impl DeviceHandle {
    /// Read a command whose entry declares an answer, and send any other one.
    /// With `refuseSecrets`, a command that takes a secret throws `secret_arg`.
    ///
    /// @param [refuseSecrets=false]
    #[napi]
    pub fn invoke<'env>(
        &self,
        env: &'env Env,
        command: String,
        #[napi(
            ts_arg_type = "Record<string, boolean | number | string | Uint8Array | Array<number> | Array<[number, number, number]>>"
        )]
        args: Option<Object<'_>>,
        refuse_secrets: Option<bool>,
    ) -> napi::Result<PromiseRaw<'env, Invoked>> {
        let supplied = conv::args(env, args.as_ref())?;
        let secrets = if refuse_secrets.unwrap_or(false) {
            Secrets::Refuse
        } else {
            Secrets::Allow
        };
        let owned = self.inner.clone();
        promise(env, async move {
            let (mode, invoked) = owned.handle().invoke(&command, supplied, secrets).await?;
            let mode = mode.to_string();
            Ok(match invoked {
                CoreInvoked::Sent(served) => Invoked {
                    id: served.id.to_string(),
                    mode,
                    command,
                    fields: None,
                },
                CoreInvoked::Read(reply) => Invoked {
                    id: reply.id.to_string(),
                    mode,
                    command,
                    fields: Some(reply.fields.to_json()),
                },
            })
        })
    }
}

/// What one verb of `Devices.apply()` answered.
#[napi]
#[derive(Clone)]
pub struct AppliedStep {
    step: &'static str,
    outcomes: Vec<Outcome>,
}

#[napi]
impl AppliedStep {
    /// The verb, in snake case: `color_temp`.
    #[napi(getter)]
    pub fn step(&self) -> &'static str {
        self.step
    }

    /// One per member that the step reached, in member order.
    #[napi(getter)]
    pub fn outcomes(&self) -> Vec<Outcome> {
        self.outcomes.clone()
    }
}

/// What `Devices.apply()` answered.
#[napi]
pub struct Applied {
    reached: Vec<Outcome>,
    steps: Vec<AppliedStep>,
    ok: bool,
    verdict: Option<String>,
}

#[napi]
impl Applied {
    /// The scan of every member, run before the verbs.
    #[napi(getter)]
    pub fn reached(&self) -> Vec<Outcome> {
        self.reached.clone()
    }

    /// One per verb, in the order sent.
    #[napi(getter)]
    pub fn steps(&self) -> Vec<AppliedStep> {
        self.steps.clone()
    }

    /// Whether every member took every step.
    #[napi(getter)]
    pub fn ok(&self) -> bool {
        self.ok
    }

    /// The line that reports the failures: `"2 of 3 devices failed: <ids>"`,
    /// or the error's own message where the set has one member. `null`
    /// where every member took every step.
    #[napi(getter)]
    pub fn verdict(&self) -> Option<String> {
        self.verdict.clone()
    }
}

impl Applied {
    fn new(applied: &CoreApplied, members: usize) -> Self {
        Self {
            reached: Outcome::all(&applied.reached),
            steps: applied
                .steps
                .iter()
                .map(|step| AppliedStep {
                    step: step.name,
                    outcomes: Outcome::all(&step.outcomes),
                })
                .collect(),
            ok: applied.is_clean(),
            verdict: applied.verdict(members).map(|(_, line)| line),
        }
    }
}

#[napi]
impl Devices {
    /// Scan for the members, then send the verbs, power on first and power
    /// off last. A member that fails takes no later step.
    #[napi]
    pub fn apply<'env>(
        &self,
        env: &'env Env,
        #[napi(
            ts_arg_type = "{ power?: boolean, brightness?: number, color?: [number, number, number] | Uint8Array, colorTemp?: number, segment?: { colors: [number, number, number] | Array<[number, number, number]> | Uint8Array, zones?: Array<number>, resolution?: number | 'app' | 'native' | 'groups', gradient?: boolean }, music?: { effect: number, sensitivity?: number, soft?: boolean, color?: [number, number, number] | Uint8Array }, gradient?: boolean }"
        )]
        verbs: Object<'_>,
    ) -> napi::Result<PromiseRaw<'env, Applied>> {
        let verbs = conv::read(env, input::verbs(&conv::options(env, Some(&verbs))?, STYLE))?;
        let set = self.inner.clone();
        promise(env, async move {
            let applied = set.devices().apply(verbs).await;
            Ok(Applied::new(&applied, set.ids().len()))
        })
    }
}
