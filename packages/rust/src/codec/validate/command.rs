//! Structural checks on one command entry.

use crate::codec::catalog::{ArgRole, Command, Mode};
use crate::codec::chunk::{self, Layout};
use crate::codec::frame::Frame;

pub(super) fn check_command(mode: Mode, name: &str, command: &Command) -> Vec<String> {
    let mut problems = Vec::new();

    problems.extend(check_arg_roles(command));
    problems.extend(check_reserved_args(command));
    problems.extend(check_documentation(mode, command));

    let frame = match command.frame.as_deref() {
        None => None,
        Some(source) => parse(name, source, &mut problems),
    };
    let chunked = check_chunk(name, command, &mut problems);

    if frame.is_some() && command.body.is_some() {
        problems.push(
            "declares both `frame:` and `body:`; a command sends one or the other".to_owned(),
        );
    }

    for layout in frame.iter().chain(chunked.iter().flat_map(Layout::frames)) {
        for arg in layout.arg_names() {
            if !command.args.contains_key(arg) && !chunk::RESERVED.contains(&arg) {
                problems.push(format!(
                    "frame refers to `{arg}`, which `args:` does not declare"
                ));
            }
        }
    }

    if command.cmd.is_empty() && frame.is_none() && chunked.is_none() {
        problems
            .push("declares neither a `cmd:` nor a frame layout, so it sends nothing".to_owned());
    }

    problems.extend(check_payload(command, frame.is_some()));
    problems
}

/// A role the SDK fills is found by that role, so at most one argument may
/// claim it, and only at the type the role is defined at.
fn check_arg_roles(command: &Command) -> Vec<String> {
    let mut problems = Vec::new();
    for arg_role in ArgRole::ALL {
        let claimants: Vec<&str> = command
            .args
            .iter()
            .filter(|(_, spec)| spec.role() == Some(arg_role))
            .map(|(arg, _)| arg.as_str())
            .collect();
        if claimants.len() > 1 {
            problems.push(format!(
                "`role: {arg_role}` is claimed by {}; at most one argument may claim a role",
                claimants.join(", ")
            ));
        }
        for arg in claimants {
            let Some(spec) = command.args.get(arg) else {
                continue;
            };
            if spec.type_name() != arg_role.type_name() {
                problems.push(format!(
                    "`{arg}` is marked `role: {arg_role}`, which has to be {}, not {}",
                    arg_role.type_name(),
                    spec.type_name()
                ));
            }
        }
    }
    problems
}

/// The codec supplies these three itself, so an argument of the same name would
/// be silently written over.
fn check_reserved_args(command: &Command) -> Vec<String> {
    command
        .args
        .keys()
        .filter(|arg| chunk::RESERVED.contains(&arg.as_str()))
        .map(|arg| format!("`{arg}` is a name the codec fills in; `args:` may not declare it"))
        .collect()
}

/// An undocumented command is only reproducible if the protocol section behind
/// it is written down. See CLAUDE.md, "Device files".
fn check_documentation(mode: Mode, command: &Command) -> Vec<String> {
    if command.documented {
        return Vec::new();
    }
    let expected = format!("docs/protocol/{mode}.md");
    if command.notes.trim().is_empty() {
        vec!["undocumented, but carries no `notes:`".to_owned()]
    } else if !command.notes.contains(&expected) {
        vec![format!(
            "undocumented `notes:` does not point at {expected}"
        )]
    } else {
        Vec::new()
    }
}

/// `body:` and `chunk:` are one declaration in two halves: neither means
/// anything alone.
fn check_chunk(name: &str, command: &Command, problems: &mut Vec<String>) -> Option<Layout> {
    match (command.body.as_deref(), command.chunk.as_ref()) {
        (None, None) => None,
        (Some(_), None) => {
            problems.push("declares `body:` but no `chunk:` to split it with".to_owned());
            None
        }
        (None, Some(_)) => {
            problems.push("declares `chunk:` but no `body:` to split".to_owned());
            None
        }
        (Some(body), Some(chunk)) => {
            for (field, layout) in [
                ("header", &chunk.header),
                ("data", &chunk.data),
                ("footer", &chunk.footer),
            ] {
                if layout.trim().is_empty() {
                    problems.push(format!("`chunk:` declares no `{field}:` layout"));
                }
            }
            match Layout::parse(name, body, chunk) {
                Ok(layout) => Some(layout),
                Err(e) => {
                    problems.push(e.to_string());
                    None
                }
            }
        }
    }
}

fn check_payload(command: &Command, has_frame: bool) -> Vec<String> {
    let mut problems = Vec::new();
    let mut placeholders = Vec::new();
    collect_placeholders(&command.payload, &mut placeholders);

    for name in &placeholders {
        if name == "frame" {
            if !has_frame {
                problems.push("payload uses `${frame}` but no `frame:` is declared".to_owned());
            }
        } else if !command.args.contains_key(name) {
            problems.push(format!(
                "payload refers to `{name}`, which `args:` does not declare"
            ));
        }
    }
    if has_frame && !placeholders.iter().any(|p| p == "frame") {
        problems.push("declares a `frame:` the payload never carries with `${frame}`".to_owned());
    }
    if command.cmd.is_empty() && !command.payload.is_null() {
        problems.push("declares a `payload:` but no `cmd:` to carry it".to_owned());
    }
    problems
}

fn parse(name: &str, source: &str, problems: &mut Vec<String>) -> Option<Frame> {
    match Frame::parse(name, source) {
        Ok(frame) => Some(frame),
        Err(e) => {
            problems.push(e.to_string());
            None
        }
    }
}

fn collect_placeholders(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::String(s) => {
            if let Some(inner) = crate::codec::command::placeholder(s) {
                out.push(inner.to_owned());
            }
        }
        serde_json::Value::Object(map) => {
            for v in map.values() {
                collect_placeholders(v, out);
            }
        }
        serde_json::Value::Array(items) => {
            for v in items {
                collect_placeholders(v, out);
            }
        }
        _ => {}
    }
}
