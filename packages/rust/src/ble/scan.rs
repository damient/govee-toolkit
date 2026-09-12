//! Finding devices: which advertised names are ours, and what one carries.
//!
//! The vendor writes the SKU into the advertised name, under one of two
//! grammars: between two underscores, or right after a `GV` prefix and before
//! four hex digits. A family advertising under another name is not found — see
//! [`Transport::bind`](super::transport::Transport::bind).
//!
//! `GBK_` and `GV` were both observed (`docs/protocol/ble.md` 1.3); the older
//! brand prefixes are reported rather than seen.

/// How many characters a SKU has, in a name that delimits it by position
/// rather than by underscores.
const SKU_LEN: usize = 5;

/// How many hex digits follow the SKU in such a name.
const TAIL_LEN: usize = 4;

/// The advertised-name prefixes recognized, matched case-insensitively.
pub const NAME_PREFIXES: [&str; 6] = ["GBK_", "GOVEE", "GVH", "GVR", "IHOMENT_", "MINGER_"];

/// One device as an advertisement describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Advertised {
    /// The handle the platform addresses this peripheral by. This is where to
    /// connect, and it is **not** the device's identity.
    ///
    /// The platform decides its shape: a Bluetooth address, or a per-host
    /// identifier on macOS, where `CoreBluetooth` reports every peripheral as
    /// `00:00:00:00:00:00`. The adapter gets it back as it came.
    pub endpoint: String,
    /// The name it advertises.
    pub name: String,
    /// The SKU read out of that name.
    pub sku: String,
}

impl Advertised {
    /// Read an advertisement, or `None` if the name is not one of ours or
    /// carries no SKU.
    ///
    /// A name with no SKU field is refused rather than reported with an empty
    /// one: nothing can be encoded for an unknown model.
    #[must_use]
    pub fn read(endpoint: impl Into<String>, name: &str) -> Option<Self> {
        let sku = sku_of(name)?;
        Some(Self {
            endpoint: endpoint.into(),
            name: name.to_owned(),
            sku,
        })
    }
}

/// Whether a name is one this transport recognizes.
///
/// This answers on the prefix alone. A name can carry a prefix and still hold
/// no SKU this can read, so [`sku_of`] is what decides whether a device was
/// found.
#[must_use]
pub fn is_govee(name: &str) -> bool {
    let upper = name.trim().to_uppercase();
    NAME_PREFIXES.iter().any(|prefix| upper.starts_with(prefix))
}

/// The SKU an advertised name carries.
///
/// Two grammars, and the name says which: an underscore-delimited name carries
/// the SKU in its second field, and a `GV` name carries it in the five
/// characters after the prefix, followed by four hex digits.
///
/// `None` for a name this transport does not recognize, and for a recognized
/// name that matches neither grammar.
#[must_use]
pub fn sku_of(name: &str) -> Option<String> {
    let name = name.trim();
    if !is_govee(name) {
        return None;
    }
    if let Some(field) = name.split('_').nth(1) {
        return (!field.is_empty()).then(|| field.to_uppercase());
    }
    positional_sku(name)
}

/// The SKU of a `GV<SKU><4 hex digits>` name.
///
/// The length is what tells this grammar from a longer name that merely starts
/// the same way, so a name of any other length carries no SKU here.
fn positional_sku(name: &str) -> Option<String> {
    let rest = name.get("GV".len()..)?;
    if !rest.is_ascii() || rest.len() != SKU_LEN + TAIL_LEN {
        return None;
    }
    let (sku, tail) = rest.split_at(SKU_LEN);
    tail.chars()
        .all(|c| c.is_ascii_hexdigit())
        .then(|| sku.to_uppercase())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use super::*;

    #[test]
    fn reads_the_sku_out_of_a_name() {
        assert_eq!(sku_of("GBK_H0004_6BAF").as_deref(), Some("H0004"));
    }

    #[test]
    fn reads_the_sku_out_of_a_positional_name() {
        assert_eq!(sku_of("GVH600812E5").as_deref(), Some("H6008"));
        assert_eq!(sku_of("GVR2A800001").as_deref(), Some("R2A80"));
    }

    #[test]
    fn every_family_prefix_is_recognized() {
        assert!(is_govee("GBK_H0004_6BAF"));
        assert!(is_govee("ihoment_H0005_1A2B"));
        assert!(is_govee("Minger_H0006_0001"));
        assert!(is_govee("Govee_H0003_ABCD"));
        assert!(is_govee("GVH600812E5"));
    }

    #[test]
    fn something_else_on_the_air_is_not_a_device() {
        assert!(!is_govee("Living Room Speaker"));
        assert!(Advertised::read("AA:BB:CC:DD:EE:FF", "Living Room Speaker").is_none());
    }

    #[test]
    fn a_recognized_name_with_no_model_field_is_not_a_device() {
        assert_eq!(sku_of("GBK_"), None);
        assert_eq!(sku_of("GOVEE"), None);
    }

    #[test]
    fn a_positional_name_of_another_length_carries_no_sku() {
        assert_eq!(sku_of("GVH6008"), None);
        assert_eq!(sku_of("GVH600812E5FF"), None);
    }

    #[test]
    fn a_positional_name_whose_tail_is_not_hex_carries_no_sku() {
        assert_eq!(sku_of("GVH6008ZZZZ"), None);
    }

    #[test]
    fn the_endpoint_is_carried_but_is_not_the_identity() {
        let seen = Advertised::read("AA:BB:CC:DD:EE:FF", "GBK_H0004_6BAF").expect("a device");
        assert_eq!(seen.endpoint, "AA:BB:CC:DD:EE:FF");
        assert_eq!(seen.sku, "H0004");
        assert_eq!(seen.name, "GBK_H0004_6BAF");
    }
}
