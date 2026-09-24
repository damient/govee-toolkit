//! Groups: a name that several devices carry under `groups:`, so one target
//! names every member.
//!
//! A group matches by name, ignoring case and exactly, as a device name does.

use super::{Config, Problem};
use crate::transport::DeviceId;

impl Config {
    /// The groups the configuration puts a device in.
    #[must_use]
    pub fn groups_for(&self, id: &DeviceId) -> &[String] {
        self.devices.get(id).map_or(&[], |d| d.groups.as_slice())
    }

    /// The members of `group`, in identity order. Empty where no device
    /// carries that group.
    #[must_use]
    pub fn members(&self, group: &str) -> Vec<DeviceId> {
        self.devices
            .iter()
            .filter(|(_, device)| carries(&device.groups, group))
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Whether a device carries `group` under `groups:`.
    #[must_use]
    pub fn is_group(&self, group: &str) -> bool {
        self.devices
            .values()
            .any(|device| carries(&device.groups, group))
    }

    /// A group that a device also carries as a name makes a bare target
    /// ambiguous, and a group listed twice on one device is a slip.
    pub(super) fn group_problems(&self) -> Vec<Problem> {
        let mut problems = Vec::new();
        for (id, device) in &self.devices {
            let mut seen: Vec<&str> = Vec::new();
            for group in &device.groups {
                if group.trim().is_empty() {
                    problems.push(Problem {
                        device: Some(id.clone()),
                        message: "groups lists an empty name".to_owned(),
                    });
                    continue;
                }
                if seen.iter().any(|other| other.eq_ignore_ascii_case(group)) {
                    problems.push(Problem {
                        device: Some(id.clone()),
                        message: format!("`{group}` is listed twice in groups"),
                    });
                }
                seen.push(group);
            }
            if let Some(name) = &device.name
                && self.is_group(name)
            {
                problems.push(Problem {
                    device: Some(id.clone()),
                    message: format!(
                        "`{name}` is the name of this device and a group; a bare `{name}` is refused, so write `name:{name}` or `group:{name}`"
                    ),
                });
            }
        }
        problems
    }
}

pub(crate) fn carries(groups: &[String], group: &str) -> bool {
    groups.iter().any(|given| given.eq_ignore_ascii_case(group))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    fn config(yaml: &str) -> Config {
        serde_norway::from_str(yaml).expect("the configuration parses")
    }

    fn rig() -> Config {
        config(
            r#"
            devices:
              "BB:00:00:00:00:02":
                name: hall
                groups: [ground-floor]
              "AA:00:00:00:00:01":
                name: kitchen
                groups: [Ground-Floor, ambient]
              "CC:00:00:00:00:03":
                name: desk
            "#,
        )
    }

    #[test]
    fn a_group_lists_its_members_in_identity_order_and_ignores_case() {
        let members = rig().members("GROUND-floor");
        assert_eq!(
            members,
            [
                DeviceId::new("AA:00:00:00:00:01"),
                DeviceId::new("BB:00:00:00:00:02")
            ]
        );
        assert!(rig().members("attic").is_empty());
        assert!(rig().is_group("ambient"));
        assert!(!rig().is_group("desk"));
    }

    #[test]
    fn a_device_with_no_groups_writes_no_key() {
        let written = serde_norway::to_string(&rig()).expect("the configuration writes");
        assert_eq!(written.matches("groups").count(), 2);
    }

    #[test]
    fn a_name_that_is_also_a_group_is_a_problem() {
        let clash = config(
            r#"
            devices:
              "AA:00:00:00:00:01":
                name: ambient
              "BB:00:00:00:00:02":
                groups: [ambient, AMBIENT, " "]
            "#,
        );
        let messages: Vec<String> = clash
            .group_problems()
            .iter()
            .map(|p| p.message.clone())
            .collect();
        assert_eq!(messages.len(), 3, "{messages:?}");
        assert!(messages.iter().any(|m| m.contains("name:ambient")));
        assert!(messages.iter().any(|m| m.contains("listed twice")));
        assert!(rig().group_problems().is_empty());
    }
}
