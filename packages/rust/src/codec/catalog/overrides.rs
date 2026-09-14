//! What a device file changes in a command an `include:` brought in.
//!
//! A shared table carries the layout, and one model differs in a bound or in
//! what a reader must know before calling it. An override patches that one
//! field, so the model keeps the shared layout rather than copying it. It
//! reaches only a command a family declares: a local command is edited where
//! it is written.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use super::bounds::Bounds;
use super::spec::ArgSpec;
use super::{Command, Mode};
use crate::codec::error::{Error, Result};

/// The patches a device file declares, by mode.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Overrides {
    /// Patches to `lan` commands.
    pub lan: BTreeMap<String, Override>,
    /// Patches to `ble` commands.
    pub ble: BTreeMap<String, Override>,
    /// Patches to `cloud` commands.
    pub cloud: BTreeMap<String, Override>,
}

impl Overrides {
    /// The patches for `mode`.
    #[must_use]
    pub fn get(&self, mode: Mode) -> &BTreeMap<String, Override> {
        match mode {
            Mode::Lan => &self.lan,
            Mode::Ble => &self.ble,
            Mode::Cloud => &self.cloud,
        }
    }
}

/// One patch, against one command.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Override {
    /// Remove the command. The hardware the family covers has it and this
    /// model does not. Mutually exclusive with every other field.
    pub drop: bool,
    /// Replace what the family says a caller must know.
    pub notes: Option<String>,
    /// Add one more thing a caller must know, after what the family says.
    /// The two join with a space. Mutually exclusive with `notes`.
    pub notes_append: Option<String>,
    /// Replace a bound, by argument name.
    pub args: BTreeMap<String, ArgOverride>,
}

/// The bound of one argument. Every field is optional, and one that does not
/// apply to the argument's type is an error rather than a value nothing reads.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ArgOverride {
    /// New bounds for an `int`.
    pub range: Option<Bounds>,
    /// New cap for an `rgb_list`, a `string` or a `bytes`.
    pub max_len: Option<usize>,
    /// New zone count for a `zones`.
    pub count: Option<usize>,
}

/// Apply `overrides` to `table`, which holds the commands of `mode`.
///
/// `local` names the commands the device file declares itself. Those are
/// refused: a file edits its own command where it writes it.
///
/// # Errors
///
/// [`Error::Override`] where the patch names a command no included table
/// carries, names a local command, names an argument the command does not
/// declare, carries a field the argument's type has no room for, or sets a
/// value the command already has.
pub fn apply(
    file: &str,
    mode: Mode,
    table: &mut BTreeMap<String, Command>,
    local: &BTreeSet<String>,
    overrides: &BTreeMap<String, Override>,
) -> Result<()> {
    for (name, patch) in overrides {
        let fail = |problem: String| Error::Override {
            file: file.to_owned(),
            mode,
            command: name.clone(),
            problem,
        };
        if local.contains(name) {
            return Err(fail(
                "this file declares the command itself; change it there".to_owned(),
            ));
        }
        let Some(command) = table.get_mut(name) else {
            return Err(fail("no table this file includes declares it".to_owned()));
        };
        if patch.drop {
            if patch.notes.is_some() || patch.notes_append.is_some() || !patch.args.is_empty() {
                return Err(fail(
                    "`drop` removes the command, so it takes no other field".to_owned(),
                ));
            }
            table.remove(name);
            continue;
        }
        apply_one(command, patch, &fail)?;
    }
    Ok(())
}

fn apply_one(
    command: &mut Command,
    patch: &Override,
    fail: &impl Fn(String) -> Error,
) -> Result<()> {
    match (&patch.notes, &patch.notes_append) {
        (Some(_), Some(_)) => {
            return Err(fail(
                "`notes` replaces the note and `notes_append` extends it, so a patch takes one \
                 or the other"
                    .to_owned(),
            ));
        }
        (Some(notes), None) => {
            if *notes == command.notes {
                return Err(fail(
                    "`notes` repeats what the table already says".to_owned(),
                ));
            }
            command.notes.clone_from(notes);
        }
        (None, Some(extra)) => {
            if command.notes.is_empty() {
                return Err(fail(
                    "the table gives no note to append to; write the whole note under `notes`"
                        .to_owned(),
                ));
            }
            if command.notes.contains(extra.as_str()) {
                return Err(fail(
                    "`notes_append` repeats what the table already says".to_owned(),
                ));
            }
            command.notes.push(' ');
            command.notes.push_str(extra);
        }
        (None, None) => {}
    }
    for (arg, bound) in &patch.args {
        let Some(spec) = command.args.get_mut(arg) else {
            return Err(fail(format!("the command declares no argument `{arg}`")));
        };
        patch_arg(spec, bound).map_err(|problem| fail(format!("argument `{arg}`: {problem}")))?;
    }
    Ok(())
}

fn patch_arg(spec: &mut ArgSpec, patch: &ArgOverride) -> std::result::Result<(), String> {
    let type_name = spec.type_name();
    match spec {
        ArgSpec::Int { range, .. } => {
            reject(patch.max_len.is_some(), "max_len", type_name)?;
            reject(patch.count.is_some(), "count", type_name)?;
            let Some(new) = &patch.range else {
                return Err("nothing to change".to_owned());
            };
            if new.pair() == range.as_ref().and_then(Bounds::pair) && new.pair().is_some() {
                return Err("`range` repeats the bounds the table already gives".to_owned());
            }
            *range = Some(new.clone());
        }
        ArgSpec::RgbList { max_len, .. }
        | ArgSpec::String { max_len, .. }
        | ArgSpec::Bytes { max_len, .. } => {
            reject(patch.range.is_some(), "range", type_name)?;
            reject(patch.count.is_some(), "count", type_name)?;
            let Some(new) = patch.max_len else {
                return Err("nothing to change".to_owned());
            };
            if *max_len == Some(new) {
                return Err("`max_len` repeats the cap the table already gives".to_owned());
            }
            *max_len = Some(new);
        }
        ArgSpec::Zones { count, .. } => {
            reject(patch.range.is_some(), "range", type_name)?;
            reject(patch.max_len.is_some(), "max_len", type_name)?;
            let Some(new) = patch.count else {
                return Err("nothing to change".to_owned());
            };
            if *count == Some(new) {
                return Err("`count` repeats the count the table already gives".to_owned());
            }
            *count = Some(new);
        }
    }
    Ok(())
}

fn reject(given: bool, field: &str, type_name: &str) -> std::result::Result<(), String> {
    if given {
        return Err(format!("`{field}` does not apply to a `{type_name}`"));
    }
    Ok(())
}
