//! `DeviceHandle.invoke()` and `GroupHandle.apply()`.

use govee_toolkit::{Applied as CoreApplied, Invoked as CoreInvoked, Music, Verb};
use napi::Env;
use napi::bindgen_prelude::{Object, PromiseRaw, Unknown};
use napi_derive::napi;

use crate::conv::{self, Channels};
use crate::device::DeviceHandle;
use crate::errors::value_error;
use crate::group::{GroupHandle, Outcome};
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
        let supplied = conv::args(env, args)?;
        let (govee, pinned, id) = self.parts();
        promise(env, async move {
            let handle = govee.device_maybe_on(&id, pinned);
            let call = handle.resolve()?;
            if refuse_secrets.unwrap_or(false) {
                call.refuse_secret(&command)?;
            }
            let values = call.args(&command, supplied)?;
            let mode = call.mode().to_string();
            Ok(match call.invoke(&command, &values).await? {
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

/// What one verb of `GroupHandle.apply()` answered.
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

/// What `GroupHandle.apply()` answered.
#[napi]
pub struct Applied {
    reached: Vec<Outcome>,
    steps: Vec<AppliedStep>,
    ok: bool,
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
}

impl From<CoreApplied> for Applied {
    fn from(applied: CoreApplied) -> Self {
        let ok = applied.is_clean();
        Self {
            reached: applied
                .reached
                .into_iter()
                .map(|outcome| Outcome::new(outcome, |mode| (mode, None)))
                .collect(),
            steps: applied
                .steps
                .into_iter()
                .map(|step| AppliedStep {
                    step: step.name,
                    outcomes: step
                        .outcomes
                        .into_iter()
                        .map(|outcome| Outcome::new(outcome, |served| (served.mode, Some(served))))
                        .collect(),
                })
                .collect(),
            ok,
        }
    }
}

#[napi]
impl GroupHandle {
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
        let verbs = read_verbs(env, &verbs)?;
        let (govee, pinned, members) = self.parts();
        env.spawn_future(async move {
            let applied = govee.group_maybe_on(&members, pinned).apply(verbs).await;
            Ok(Applied::from(applied))
        })
    }
}

const VERBS: [&str; 7] = [
    "power",
    "brightness",
    "color",
    "colorTemp",
    "segment",
    "music",
    "gradient",
];
const SEGMENT: [&str; 4] = ["colors", "zones", "resolution", "gradient"];
const MUSIC: [&str; 4] = ["effect", "sensitivity", "soft", "color"];

/// A misspelled key is refused: ignored, it reads as a setting that failed.
fn known_keys(env: &Env, object: &Object<'_>, what: &str, keys: &[&str]) -> napi::Result<()> {
    for key in Object::keys(object)? {
        if !keys.contains(&key.as_str()) {
            return Err(value_error(
                env,
                format!("`{key}` is no {what} key; the keys are {}", keys.join(", ")),
            ));
        }
    }
    Ok(())
}

fn read_verbs(env: &Env, object: &Object<'_>) -> napi::Result<Vec<Verb>> {
    known_keys(env, object, "verb", &VERBS)?;
    let mut verbs = Vec::new();
    // `undefined` and `null` read as absent.
    if let Some(on) = object.get::<Option<bool>>("power")?.flatten() {
        verbs.push(Verb::Power(on));
    }
    if let Some(level) = object.get::<Option<i64>>("brightness")?.flatten() {
        verbs.push(Verb::Brightness(level));
    }
    if let Some(color) = object.get::<Option<Channels<'_>>>("color")?.flatten() {
        verbs.push(Verb::Color(conv::rgb(env, &color)?));
    }
    if let Some(kelvin) = object.get::<Option<i64>>("colorTemp")?.flatten() {
        verbs.push(Verb::ColorTemp(kelvin));
    }
    if let Some(segment) = object.get::<Option<Object<'_>>>("segment")?.flatten() {
        verbs.push(read_segment(env, &segment)?);
    }
    if let Some(music) = object.get::<Option<Object<'_>>>("music")?.flatten() {
        verbs.push(Verb::Music(read_music(env, &music)?));
    }
    if let Some(on) = object.get::<Option<bool>>("gradient")?.flatten() {
        verbs.push(Verb::Gradient(on));
    }
    Ok(verbs)
}

fn read_segment(env: &Env, segment: &Object<'_>) -> napi::Result<Verb> {
    known_keys(env, segment, "segment", &SEGMENT)?;
    let Some(colors) = segment.get::<Option<Channels<'_>>>("colors")?.flatten() else {
        return Err(value_error(env, "a segment carries `colors`"));
    };
    let resolution = segment.get::<Option<Unknown<'_>>>("resolution")?.flatten();
    Ok(Verb::Segment {
        zones: segment.get::<Option<Vec<u16>>>("zones")?.flatten(),
        colors: conv::colors(env, &colors)?,
        resolution: conv::resolution_or_default(env, resolution.as_ref())?,
        gradient: segment
            .get::<Option<bool>>("gradient")?
            .flatten()
            .unwrap_or(false),
    })
}

fn read_music(env: &Env, music: &Object<'_>) -> napi::Result<Music> {
    known_keys(env, music, "music", &MUSIC)?;
    let Some(effect) = music.get::<Option<i64>>("effect")?.flatten() else {
        return Err(value_error(env, "a music verb carries `effect`"));
    };
    let color = music.get::<Option<Channels<'_>>>("color")?.flatten();
    conv::music(
        env,
        effect,
        music.get::<Option<i64>>("sensitivity")?.flatten(),
        music.get::<Option<bool>>("soft")?.flatten(),
        color.as_ref(),
    )
}
