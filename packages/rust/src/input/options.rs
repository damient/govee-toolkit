use super::values::{colors, mode, painting, rgb, seconds, whole};
use super::{Refused, Result, Value, key};
use crate::apply::Verb;
use crate::codec::Mode;
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

/// Read the verbs that [`Devices::apply`](crate::Devices::apply) sorts and
/// sends.
///
/// # Errors
///
/// [`Refused`] for an unknown key, a missing key or a bad shape.
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
    let at = |name| value.get(name).unwrap_or(&Value::Null);
    Ok(Verb::Segment(painting(
        colors(painted, style)?,
        at("zones"),
        at("resolution"),
        value.get("gradient").map(flag).transpose()?,
    )?))
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

/// Read a map of names, in the order of `keys`. `Null` reads as no key.
///
/// # Errors
///
/// [`Refused`] for a key outside `keys`, or a value that is no text.
pub fn names<'a, const N: usize>(
    value: &'a Value,
    what: &str,
    keys: [&str; N],
) -> Result<[Option<&'a str>; N]> {
    let mut names = [None; N];
    if matches!(value, Value::Null) {
        return Ok(names);
    }
    value.known_keys(what, &keys)?;
    for (slot, key) in names.iter_mut().zip(keys) {
        *slot = value
            .get(key)
            .map(|name| text(name).map_err(|_| Refused::new(format!("`{key}` is a name"))))
            .transpose()?;
    }
    Ok(names)
}

/// [`names`], where every name is a mode.
///
/// # Errors
///
/// [`Refused`] as [`names`] refuses, or for a name that is no mode.
pub fn modes<const N: usize>(
    value: &Value,
    what: &str,
    keys: [&str; N],
) -> Result<[Option<Mode>; N]> {
    let mut modes = [None; N];
    for (slot, name) in modes.iter_mut().zip(names(value, what, keys)?) {
        *slot = name.map(mode).transpose()?;
    }
    Ok(modes)
}

/// Read the options of `Govee.identify()`. `Null` reads as the defaults of
/// [`Walk`].
///
/// # Errors
///
/// [`Refused`] for an unknown key or mode, a negative duration or a bad shape.
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
