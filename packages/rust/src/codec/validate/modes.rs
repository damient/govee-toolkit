//! What a mode reaches, checked against the capability set it draws from.

use crate::codec::catalog::{Device, Mode, Support};

/// A mode's reach, checked against the capability set it draws from.
///
/// The two lists are one statement in two halves: every capability the hardware
/// has is either reached or explained. A mode nobody probed
/// ([`Support::Unknown`]) is exempt — an empty answer there is the honest one —
/// and [`Support::None`] reaches nothing by definition.
pub(super) fn check_mode_capabilities(device: &Device, mode: Mode) -> Vec<(&'static str, String)> {
    let mut problems = Vec::new();
    let support = device.modes.get(mode);
    let reached = support.capabilities.resolve(&device.capabilities);

    for name in &reached {
        if !device.capabilities.has(name) {
            problems.push((
                "capabilities",
                format!("`{name}` is not a capability this device declares"),
            ));
        }
    }
    for (name, reason) in &support.unreachable {
        if !device.capabilities.has(name) {
            problems.push((
                "unreachable",
                format!("`{name}` is not a capability this device declares"),
            ));
        } else if reached.contains(&name.as_str()) {
            problems.push((
                "unreachable",
                format!("`{name}` is also listed as reachable; it is `{reason}` or it is not"),
            ));
        }
    }

    match support.support {
        Support::None if !reached.is_empty() => problems.push((
            "capabilities",
            "reaches capabilities, but the mode is declared `none`".to_owned(),
        )),
        Support::Full if !support.unreachable.is_empty() => problems.push((
            "support",
            "is `full`, but capabilities are listed unreachable".to_owned(),
        )),
        Support::Partial if support.unreachable.is_empty() => problems.push((
            "support",
            "is `partial`, but nothing is listed unreachable".to_owned(),
        )),
        _ => {}
    }

    if matches!(support.support, Support::Full | Support::Partial) {
        let unaccounted: Vec<&str> = device
            .capabilities
            .names()
            .filter(|name| !reached.contains(name) && !support.unreachable.contains_key(*name))
            .collect();
        if !unaccounted.is_empty() {
            problems.push((
                "capabilities",
                format!(
                    "says nothing about {}; a probed mode either reaches a capability or lists it under `unreachable` with a reason",
                    unaccounted.join(", ")
                ),
            ));
        }
    }

    problems
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use crate::codec::{Catalog, validate};

    fn problems(capabilities: &str, lan: &str) -> Vec<String> {
        let file = format!(
            "schema_version: 1\nsku: HTEST\nfamily: test\nname: Test\n\
             capabilities:\n{capabilities}modes:\n  lan:\n{lan}"
        );
        let catalog =
            Catalog::from_sources([("HTEST.yaml", file.as_str())]).expect("the device file parses");
        let device = catalog.device("HTEST").expect("the SKU resolves");
        validate::device(device)
            .into_iter()
            .map(|problem| problem.message)
            .collect()
    }

    #[test]
    fn a_probed_mode_accounts_for_every_capability() {
        let found = problems(
            "  power:\n  scenes:\n",
            "    support: partial\n    capabilities: [power]\n\
             \n    unreachable: {}\n",
        );
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(
            found
                .iter()
                .any(|m| m.contains("says nothing about scenes"))
        );
        assert!(
            found
                .iter()
                .any(|m| m.contains("nothing is listed unreachable"))
        );
    }

    #[test]
    fn an_explained_capability_settles_it() {
        let found = problems(
            "  power:\n  scenes:\n",
            "    support: partial\n    capabilities: [power]\n\
             \n    unreachable:\n      scenes: transport\n",
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_mode_may_not_name_a_capability_the_device_lacks() {
        let found = problems(
            "  power:\n",
            "    support: full\n    capabilities: [power, scenes]\n",
        );
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("`scenes` is not a capability"));
    }

    #[test]
    fn a_capability_is_reached_or_explained_but_not_both() {
        let found = problems(
            "  power:\n",
            "    support: full\n    capabilities: [power]\n\
             \n    unreachable:\n      power: unprobed\n",
        );
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(found.iter().any(|m| m.contains("also listed as reachable")));
        assert!(found.iter().any(|m| m.contains("is `full`")));
    }

    #[test]
    fn an_unprobed_mode_owes_no_answer() {
        let found = problems("  power:\n  scenes:\n", "    support: unknown\n");
        assert!(found.is_empty(), "{found:?}");
    }
}
