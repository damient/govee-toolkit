//! What the core reports, as JavaScript objects.
//!
//! Every one holds the core value and reads the answers off it. No field and
//! no rule is written twice.

use std::collections::HashMap;

use govee_toolkit::summary::{Style, Summary};
use govee_toolkit::transport::{Health as CoreHealth, Reply as CoreReply};
use govee_toolkit::{Device as CoreDevice, DeviceStatus as CoreStatus, Served as CoreServed};
use napi::Env;
use napi_derive::napi;

use crate::conv::to_js;

/// A device's health in one mode.
#[napi]
pub struct Health {
    inner: CoreHealth,
}

#[napi]
impl Health {
    /// `"ok"`, `"degraded"` or `"down"`.
    #[napi(getter)]
    pub fn state(&self) -> String {
        self.inner.state.to_string()
    }

    /// Consecutive unanswered verifications.
    #[napi(getter)]
    pub fn failures(&self) -> u32 {
        self.inner.failures
    }

    /// Whether a command would be sent right now.
    #[napi(getter)]
    pub fn available(&self) -> bool {
        self.inner.available
    }

    #[napi(js_name = "toString")]
    pub fn to_js_string(&self) -> String {
        self.inner.summary(Style::Javascript)
    }
}

impl From<CoreHealth> for Health {
    fn from(inner: CoreHealth) -> Self {
        Self { inner }
    }
}

/// A device the SDK knows about.
#[napi]
pub struct Device {
    inner: CoreDevice,
}

#[napi]
impl Device {
    /// The MAC it reports, uppercased.
    #[napi(getter)]
    pub fn id(&self) -> String {
        self.inner.id.to_string()
    }

    /// The SKU it is encoded under.
    #[napi(getter)]
    pub fn sku(&self) -> String {
        self.inner.sku.clone()
    }

    /// The name the configuration gives it, if any.
    #[napi(getter)]
    pub fn name(&self) -> Option<String> {
        self.inner.name.clone()
    }

    /// The enabled modes, in preference order.
    #[napi(getter)]
    pub fn modes(&self) -> Vec<String> {
        self.inner.modes.iter().map(ToString::to_string).collect()
    }

    /// Its health per enabled mode. A mode is absent when no transport has
    /// heard from it.
    #[napi(getter)]
    pub fn health(&self) -> HashMap<String, Health> {
        self.inner
            .health
            .iter()
            .map(|(mode, health)| (mode.to_string(), Health::from(*health)))
            .collect()
    }

    #[napi(js_name = "toString")]
    pub fn to_js_string(&self) -> String {
        self.inner.summary(Style::Javascript)
    }
}

impl From<CoreDevice> for Device {
    fn from(inner: CoreDevice) -> Self {
        Self { inner }
    }
}

/// A command that was served.
#[napi]
pub struct Served {
    inner: CoreServed,
}

#[napi]
impl Served {
    /// The device it went to.
    #[napi(getter)]
    pub fn id(&self) -> String {
        self.inner.id.to_string()
    }

    /// The mode that served it.
    #[napi(getter)]
    pub fn mode(&self) -> String {
        self.inner.mode.to_string()
    }

    /// The device file entry that was sent.
    #[napi(getter)]
    pub fn command(&self) -> String {
        self.inner.command.clone()
    }

    /// The name the wire carries, where it carries one.
    #[napi(getter)]
    pub fn cmd(&self) -> String {
        self.inner.cmd.clone()
    }

    #[napi(js_name = "toString")]
    pub fn to_js_string(&self) -> String {
        self.inner.summary(Style::Javascript)
    }
}

impl From<CoreServed> for Served {
    fn from(inner: CoreServed) -> Self {
        Self { inner }
    }
}

/// What a device reported about itself. No firmware fills every field.
#[napi]
pub struct DeviceStatus {
    inner: CoreStatus,
}

#[napi]
impl DeviceStatus {
    /// Which device answered.
    #[napi(getter)]
    pub fn id(&self) -> String {
        self.inner.id.to_string()
    }

    /// Whether it is on.
    #[napi(getter)]
    pub fn on(&self) -> Option<bool> {
        self.inner.on
    }

    /// The level it reports. A percentage on every unit seen so far, and not
    /// normalized here.
    #[napi(getter)]
    pub fn brightness(&self) -> Option<i64> {
        self.inner.brightness
    }

    /// The color, as three channels. Reset to `[0, 0, 0]` in white mode.
    #[napi(getter, ts_return_type = "[number, number, number] | null")]
    pub fn color(&self) -> Option<Vec<u8>> {
        self.inner.color.map(|channels| channels.to_vec())
    }

    /// The white temperature. `0` means the device is in color mode.
    #[napi(getter)]
    pub fn color_temp_kelvin(&self) -> Option<i64> {
        self.inner.color_temp_kelvin
    }

    /// The whole reply, with every field the SDK does not model.
    #[napi(getter)]
    pub fn raw(&self, env: &Env) -> napi::Result<serde_json::Value> {
        to_js(env, &self.inner.raw)
    }

    /// Whether the device is in white mode. Mutually exclusive with color.
    #[napi(getter)]
    pub fn is_white(&self) -> bool {
        self.inner.is_white()
    }

    #[napi(js_name = "toString")]
    pub fn to_js_string(&self) -> String {
        self.inner.summary(Style::Javascript)
    }
}

impl From<CoreStatus> for DeviceStatus {
    fn from(inner: CoreStatus) -> Self {
        Self { inner }
    }
}

/// What one command's `reply:` layouts captured.
#[napi]
pub struct Reply {
    inner: CoreReply,
}

#[napi]
impl Reply {
    /// Which device answered.
    #[napi(getter)]
    pub fn id(&self) -> String {
        self.inner.id.to_string()
    }

    /// Every field the exchanges captured, by the name the device file gives
    /// it.
    #[napi(getter)]
    pub fn fields(&self, env: &Env) -> napi::Result<serde_json::Value> {
        to_js(env, &self.inner.fields.to_json())
    }

    #[napi(js_name = "toString")]
    pub fn to_js_string(&self) -> String {
        self.inner.summary(Style::Javascript)
    }
}

impl From<CoreReply> for Reply {
    fn from(inner: CoreReply) -> Self {
        Self { inner }
    }
}
