//! A target of `govee-dmx`: the grammar [`Selector`] reads, plus the names and
//! the groups of the patch. The rules are `docs/dmx.md` 4.3.

use govee_toolkit::codec::Mode;
use govee_toolkit::exit::Failure;
use govee_toolkit::select::Names;
use govee_toolkit::{Config, DeviceId, Govee, Selector, select};
use govee_toolkit_dmx::patch::{Fixture, Rig};

/// The devices `targets` name, in the order they were written, each one once.
/// A group the patch gives answers its fixtures in patch order.
///
/// # Errors
///
/// [`Failure::config`] for every [`select::Error`], and for a name or a group
/// that the patch and the configuration give to different devices.
pub(crate) fn select(
    govee: &Govee,
    rig: &Rig,
    targets: &[String],
) -> Result<Vec<DeviceId>, Failure> {
    let mut chosen: Vec<DeviceId> = Vec::new();
    for target in targets {
        for id in one(govee, rig, target)? {
            if !chosen.contains(&id) {
                chosen.push(id);
            }
        }
    }
    Ok(chosen)
}

/// The patch and the configuration, as one source of names and groups.
struct Sources<'a> {
    rig: &'a Rig,
    config: &'a Config,
}

impl Names for Sources<'_> {
    fn names(&self, name: &str) -> bool {
        !self.rig.named(name).is_empty() || self.config.names(name)
    }

    fn groups(&self, group: &str) -> bool {
        !self.rig.grouped(group).is_empty() || self.config.groups(group)
    }
}

fn one(govee: &Govee, rig: &Rig, target: &str) -> Result<Vec<DeviceId>, Failure> {
    let refused = |e: select::Error| Failure::config(e.to_string());
    let config = govee.config();
    let selector =
        Selector::resolve(target, govee.catalog(), &Sources { rig, config }).map_err(refused)?;
    match &selector {
        Selector::Name(name) => {
            let named = ids(&rig.named(name));
            if !named.is_empty() {
                let configured: Vec<DeviceId> = config.named(name).cloned().collect();
                return agree(named, &configured, "name", name);
            }
        }
        Selector::Group(group) => {
            let grouped = ids(&rig.grouped(group));
            if !grouped.is_empty() {
                return agree(grouped, &config.members(group), "group", group);
            }
        }
        Selector::Id(_) | Selector::Sku(_) => {}
    }
    // The prefixed form: the scanned devices do not settle the target again.
    govee
        .select([selector.to_string()], Some(Mode::Lan))
        .map_err(refused)
}

/// `patched`, where the configuration gives `value` to no device outside it.
fn agree(
    patched: Vec<DeviceId>,
    configured: &[DeviceId],
    kind: &str,
    value: &str,
) -> Result<Vec<DeviceId>, Failure> {
    match configured.iter().find(|id| !patched.contains(id)) {
        Some(other) => Err(Failure::config(format!(
            "`{kind}:{value}` names fixtures of the patch, and the device {other} in the configuration; write `id:…`"
        ))),
        None => Ok(patched),
    }
}

fn ids(fixtures: &[&Fixture]) -> Vec<DeviceId> {
    fixtures
        .iter()
        .map(|fixture| fixture.entry.device.clone())
        .collect()
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use std::sync::Arc;

    use govee_toolkit::codec::Catalog;
    use govee_toolkit::{Config, Transport};
    use govee_toolkit_dmx::patch::Patch;

    use super::*;

    const KITCHEN: &str = "AA:BB:CC:DD:EE:01";
    const HALL: &str = "AA:BB:CC:DD:EE:02";
    const SHELF: &str = "AA:BB:CC:DD:EE:00";
    const OTHER: &str = "AA:BB:CC:DD:EE:09";

    const PATCH: &str = r#"
patch:
  - device: "AA:BB:CC:DD:EE:01"
    name: kitchen
    groups: [bar]
    universe: 0
    address: 1
    personality: full
  - device: "AA:BB:CC:DD:EE:02"
    name: H6008
    groups: [Bar]
    universe: 0
    address: 7
    personality: full
  - device: "AA:BB:CC:DD:EE:00"
    groups: [bar, hall]
    universe: 0
    address: 13
    personality: full
"#;

    fn govee(config: &str) -> Govee {
        let config: Config = serde_norway::from_str(config).expect("the configuration parses");
        let catalog = Catalog::embedded().expect("the embedded catalog parses");
        Govee::attach(config, catalog, std::iter::empty::<Arc<dyn Transport>>())
            .expect("the SDK attaches")
    }

    fn rig(govee: &Govee) -> Rig {
        let patch = Patch::parse(PATCH, "patch.yaml").unwrap_or_else(|e| panic!("{e}"));
        let device = govee.catalog().device("H6008").expect("the SKU resolves");
        patch
            .resolve(|_| Some(device), |_| Some(device))
            .unwrap_or_else(|errors| panic!("{errors:?}"))
    }

    fn chosen(config: &str, targets: &[&str]) -> Result<Vec<String>, String> {
        let govee = govee(config);
        let targets: Vec<String> = targets.iter().map(|t| (*t).to_owned()).collect();
        select(&govee, &rig(&govee), &targets)
            .map(|ids| ids.iter().map(ToString::to_string).collect())
            .map_err(|failure| format!("{failure:?}"))
    }

    #[tokio::test]
    async fn a_name_the_patch_gives_selects_its_fixture() {
        assert_eq!(chosen("{}", &["kitchen"]), Ok(vec![KITCHEN.to_owned()]));
        assert_eq!(
            chosen("{}", &["name:KITCHEN"]),
            Ok(vec![KITCHEN.to_owned()])
        );
    }

    #[tokio::test]
    async fn the_name_of_the_patch_wins_over_another_name_in_the_configuration() {
        let config = format!("devices:\n  \"{KITCHEN}\":\n    name: cuisine\n");
        assert_eq!(chosen(&config, &["kitchen"]), Ok(vec![KITCHEN.to_owned()]));
    }

    #[tokio::test]
    async fn a_name_of_two_devices_is_refused() {
        let config = format!("devices:\n  \"{HALL}\":\n    name: kitchen\n");
        let refused = chosen(&config, &["kitchen"]).expect_err("two devices carry `kitchen`");
        assert!(refused.contains(HALL), "{refused}");
        assert_eq!(
            chosen(&config, &[&format!("id:{KITCHEN}")]),
            Ok(vec![KITCHEN.to_owned()])
        );
    }

    #[tokio::test]
    async fn a_bare_sku_that_a_fixture_carries_as_a_name_is_refused() {
        let refused = chosen("{}", &["H6008"]).expect_err("`H6008` reads as two kinds");
        assert!(refused.contains("reads as a sku"), "{refused}");
        assert_eq!(chosen("{}", &["name:H6008"]), Ok(vec![HALL.to_owned()]));
    }

    #[tokio::test]
    async fn a_name_of_the_patch_that_is_also_a_group_is_refused() {
        let config = format!("devices:\n  \"{HALL}\":\n    groups: [kitchen]\n");
        let refused = chosen(&config, &["kitchen"]).expect_err("`kitchen` reads as two kinds");
        assert!(refused.contains("group:kitchen"), "{refused}");
        assert_eq!(
            chosen(&config, &["name:kitchen"]),
            Ok(vec![KITCHEN.to_owned()])
        );
    }

    #[tokio::test]
    async fn a_group_the_patch_gives_selects_its_fixtures_in_patch_order() {
        let bar = vec![KITCHEN.to_owned(), HALL.to_owned(), SHELF.to_owned()];
        assert_eq!(chosen("{}", &["bar"]), Ok(bar.clone()));
        assert_eq!(chosen("{}", &["group:BAR"]), Ok(bar.clone()));
        let config = format!("devices:\n  \"{KITCHEN}\":\n    groups: [bar]\n");
        assert_eq!(chosen(&config, &["bar"]), Ok(bar));
    }

    #[tokio::test]
    async fn a_group_of_the_configuration_beyond_the_patch_is_refused() {
        let config = format!("devices:\n  \"{OTHER}\":\n    groups: [bar]\n");
        let refused = chosen(&config, &["bar"]).expect_err("the two disagree on `bar`");
        assert!(refused.contains(OTHER), "{refused}");
        assert_eq!(
            chosen(&config, &[&format!("id:{SHELF}")]),
            Ok(vec![SHELF.to_owned()])
        );
    }

    #[tokio::test]
    async fn a_group_of_the_patch_that_is_also_a_name_is_refused_when_bare() {
        let config = format!("devices:\n  \"{OTHER}\":\n    name: hall\n");
        let refused = chosen(&config, &["hall"]).expect_err("`hall` reads as two kinds");
        assert!(refused.contains("group:hall"), "{refused}");
        assert_eq!(chosen(&config, &["group:hall"]), Ok(vec![SHELF.to_owned()]));
    }
}
