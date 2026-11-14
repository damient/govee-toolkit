//! The option maps: the verbs of `Devices.apply()` and the walk of
//! `Govee.identify()`.

use super::values::{colors, mode, resolution, rgb, seconds, whole, zones};
use super::{Refused, Result, Value, key};
use crate::apply::Verb;
use crate::govee::Walk;
use crate::summary::Style;
use crate::verbs::Music;

const VERBS: [&str; 7] = [
    "power",
    "brightness",
    "color",
    "color_temp",
    "segment",
    "music",
    "gradient",
];
const SEGMENT: [&str; 4] = ["colors", "zones", "resolution", "gradient"];
const MUSIC: [&str; 4] = ["effect", "sensitivity", "soft", "color"];
const WALK: [&str; 5] = ["color", "wait", "hold", "keep", "mode"];

fn flag(value: &Value) -> Result<bool> {
    match value {
        Value::Bool(flag) => Ok(*flag),
        _ => Err(Refused::new("a boolean is expected")),
    }
}

fn text(value: &Value) -> Result<&str> {
    match value {
        Value::Text(text) => Ok(text),
        _ => Err(Refused::new("a name is expected")),
    }
}

fn known(value: &Value, what: &str, keys: &[&str], style: Style) -> Result<()> {
    let spelled: Vec<String> = keys.iter().map(|k| key(style, k)).collect();
    let spelled: Vec<&str> = spelled.iter().map(String::as_str).collect();
    value.known_keys(what, &spelled)
}

/// Read the verbs of `Devices.apply()`, keyed as the host spells them. The
/// order is [`Devices::apply`](crate::Devices::apply)'s to set.
///
/// # Errors
///
/// [`Refused`] for an unknown key, a segment with no `colors`, a music verb
/// with no `effect`, or a value of the wrong shape.
pub fn verbs(value: &Value, style: Style) -> Result<Vec<Verb>> {
    known(value, "verb", &VERBS, style)?;
    let at = |name: &str| value.get(&key(style, name));
    let mut verbs = Vec::new();
    if let Some(on) = at("power") {
        verbs.push(Verb::Power(flag(on)?));
    }
    if let Some(level) = at("brightness") {
        verbs.push(Verb::Brightness(whole(level)?));
    }
    if let Some(color) = at("color") {
        verbs.push(Verb::Color(rgb(color, style)?));
    }
    if let Some(kelvin) = at("color_temp") {
        verbs.push(Verb::ColorTemp(whole(kelvin)?));
    }
    if let Some(segment) = at("segment") {
        verbs.push(self::segment(segment, style)?);
    }
    if let Some(music) = at("music") {
        verbs.push(Verb::Music(self::music(music, style)?));
    }
    if let Some(on) = at("gradient") {
        verbs.push(Verb::Gradient(flag(on)?));
    }
    Ok(verbs)
}

fn segment(value: &Value, style: Style) -> Result<Verb> {
    known(value, "segment", &SEGMENT, style)?;
    let Some(painted) = value.get("colors") else {
        return Err(Refused::new("a segment carries `colors`"));
    };
    Ok(Verb::Segment {
        zones: value.get("zones").map(zones).transpose()?,
        colors: colors(painted, style)?,
        resolution: resolution(value.get("resolution").unwrap_or(&Value::Null))?,
        gradient: value
            .get("gradient")
            .map(flag)
            .transpose()?
            .unwrap_or(false),
    })
}

fn music(value: &Value, style: Style) -> Result<Music> {
    known(value, "music", &MUSIC, style)?;
    let Some(effect) = value.get("effect") else {
        return Err(Refused::new("a music verb carries `effect`"));
    };
    Ok(Music::new(
        whole(effect)?,
        value.get("sensitivity").map(whole).transpose()?,
        value.get("soft").map(flag).transpose()?,
        value.get("color").map(|c| rgb(c, style)).transpose()?,
    ))
}

/// Read the options of `Govee.identify()`. `Null` reads as the defaults of
/// [`Walk`].
///
/// # Errors
///
/// [`Refused`] for an unknown key, a negative duration, an unknown mode, or a
/// value of the wrong shape.
pub fn walk(value: &Value, style: Style) -> Result<Walk> {
    let mut walk = Walk::default();
    if matches!(value, Value::Null) {
        return Ok(walk);
    }
    known(value, "walk", &WALK, style)?;
    if let Some(color) = value.get("color") {
        walk.pass.color = rgb(color, style)?;
    }
    if let Some(wait) = value.get("wait") {
        walk.wait = seconds("wait", wait)?;
    }
    if let Some(hold) = value.get("hold") {
        walk.hold = seconds("hold", hold)?;
    }
    if let Some(keep) = value.get("keep") {
        walk.keep = flag(keep)?;
    }
    if let Some(name) = value.get("mode") {
        walk.mode = mode(text(name)?)?;
    }
    Ok(walk)
}
