//! What a `cloud` entry may declare, and what only that mode may. Every rule
//! here is one the transport relies on.

use crate::codec::catalog::{Command, Mode, Role};

pub(super) fn check_cloud(mode: Mode, command: &Command) -> Vec<String> {
    let mut problems = Vec::new();
    if mode != Mode::Cloud {
        for (field, declared) in [
            ("capability", command.capability.is_some()),
            ("reads", !command.reads.is_empty()),
        ] {
            if declared {
                problems.push(format!("declares a `{field}:`, which only `cloud` carries"));
            }
        }
        return problems;
    }

    for (field, declared) in [
        ("cmd", !command.cmd.is_empty()),
        ("frame", command.frame.is_some()),
        ("frames", !command.frames.is_empty()),
        ("body", command.body.is_some()),
        ("reply", command.reply.is_some()),
    ] {
        if declared {
            problems.push(format!(
                "declares a `{field}:`, which the cloud wire does not carry"
            ));
        }
    }

    match (command.capability.is_some(), command.reads.is_empty()) {
        (true, false) => problems.push(
            "declares both a `capability:` and `reads:`; an entry writes one or reads back"
                .to_owned(),
        ),
        (false, true) => problems
            .push("declares neither a `capability:` nor `reads:`, so it sends nothing".to_owned()),
        (true, true) if command.payload.is_null() => problems
            .push("declares a `capability:` but no `payload:` to give it a value".to_owned()),
        _ => {}
    }

    if !command.reads.is_empty() && command.role != Some(Role::Status) {
        problems.push(
            "declares `reads:` but not `role: status`; nothing would ask for the answer".to_owned(),
        );
    }
    for read in &command.reads {
        let arg = &read.arg;
        match command.args.get(arg) {
            None => problems.push(format!(
                "reads `{}` into `{arg}`, which `args:` does not declare",
                read.instance
            )),
            Some(spec) if spec.role().is_none() => problems.push(format!(
                "reads `{}` into `{arg}`, which declares no `role:`; \
                 the answer would reach nothing",
                read.instance
            )),
            Some(_) => {}
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use crate::codec::{Catalog, Mode};

    fn problems(commands: &str) -> Vec<String> {
        let file = format!(
            "schema_version: 1\nsku: HTEST\nfamily: test\nname: Test\n\
             capabilities: {{}}\ncommands:\n  cloud:\n{commands}"
        );
        let catalog =
            Catalog::from_sources([("HTEST.yaml", file.as_str())]).expect("the device file parses");
        let device = catalog.device("HTEST").expect("the SKU resolves");
        crate::codec::validate::device(device)
            .into_iter()
            .map(|problem| problem.message)
            .collect()
    }

    const POWER: &str = "    power:\n      \
        capability: { type: devices.capabilities.on_off, instance: powerSwitch }\n      \
        payload: \"${on}\"\n      args:\n        on: { type: int, range: [0, 1] }\n";

    #[test]
    fn one_capability_and_its_value_is_a_whole_entry() {
        assert!(problems(POWER).is_empty());
    }

    #[test]
    fn an_entry_that_names_no_capability_and_reads_nothing_sends_nothing() {
        let found = problems("    power:\n");
        assert!(
            found.iter().any(|p| p.contains("sends nothing")),
            "{found:?}"
        );
    }

    #[test]
    fn a_frame_belongs_to_another_wire() {
        let found = problems("    power:\n      frame: \"AA 01 <pad:20> <xor>\"\n");
        assert!(
            found
                .iter()
                .any(|p| p.contains("the cloud wire does not carry")),
            "{found:?}"
        );
    }

    #[test]
    fn an_answer_needs_an_argument_with_a_role_to_land_in() {
        let found = problems(
            "    status:\n      role: status\n      \
             reads:\n        - { instance: powerSwitch, arg: on }\n      \
             args:\n        on: { type: int, range: [0, 1] }\n",
        );
        assert!(
            found.iter().any(|p| p.contains("declares no `role:`")),
            "{found:?}"
        );
    }

    #[test]
    fn only_cloud_declares_a_capability() {
        let file = "schema_version: 1\nsku: HTEST\nfamily: test\nname: Test\n\
             capabilities: {}\ncommands:\n  lan:\n    power:\n      cmd: turn\n      \
             capability: { type: devices.capabilities.on_off, instance: powerSwitch }\n";
        let catalog = Catalog::from_sources([("HTEST.yaml", file)]).expect("it parses");
        let device = catalog.device("HTEST").expect("the SKU resolves");
        let found: Vec<String> = crate::codec::validate::device(device)
            .into_iter()
            .map(|problem| problem.message)
            .collect();
        assert!(
            found.iter().any(|p| p.contains("only `cloud` carries")),
            "{found:?}"
        );
        assert_eq!(Mode::Cloud.to_string(), "cloud");
    }
}
