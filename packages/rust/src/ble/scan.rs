//! Finding devices: which advertised names are ours, and what one carries.
//!
//! An advertisement is all there is to go on before connecting, and the vendor
//! puts the SKU in the name: three underscore-separated fields, of which the
//! second is the model. A family that advertises under some other name is
//! simply not found, which is why
//! [`Transport::bind`](super::transport::Transport::bind) exists.
//!
//! Unverified in this repository: no scan has been recorded here, so both the
//! name shape and the prefix list are the shape this transport is written to.
//! TODO: record what devices actually advertise in `docs/protocol/ble.md`.
//!
//! Nothing here touches an adapter. The transport does that.

/// The advertised-name prefixes a Govee device is recognized by, matched
/// case-insensitively. `IHOMENT` and `MINGER` are the vendor's other brands.
/// Unverified: see the module documentation.
pub const NAME_PREFIXES: [&str; 4] = ["GBK_", "GOVEE", "IHOMENT_", "MINGER_"];

/// One device as an advertisement describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Advertised {
    /// The Bluetooth address it advertises from, uppercase and colon-separated.
    /// This is where to connect, and it is **not** the device's identity.
    pub address: String,
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
    /// one: nothing can be encoded for a device whose model is unknown, and a
    /// device that only appears in a listing is worse than one that does not
    /// appear at all.
    #[must_use]
    pub fn read(address: impl Into<String>, name: &str) -> Option<Self> {
        let sku = sku_of(name)?;
        Some(Self {
            address: address.into(),
            name: name.to_owned(),
            sku,
        })
    }
}

/// Whether a name is one this transport recognizes.
#[must_use]
pub fn is_govee(name: &str) -> bool {
    let upper = name.trim().to_uppercase();
    NAME_PREFIXES.iter().any(|prefix| upper.starts_with(prefix))
}

/// The SKU an advertised name carries: its second underscore-separated field.
///
/// `None` for a name this transport does not recognize, and for a recognized
/// name with no second field.
#[must_use]
pub fn sku_of(name: &str) -> Option<String> {
    if !is_govee(name) {
        return None;
    }
    let sku = name.trim().split('_').nth(1)?;
    (!sku.is_empty()).then(|| sku.to_uppercase())
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
    fn every_family_prefix_is_recognized() {
        assert!(is_govee("GBK_H0004_6BAF"));
        assert!(is_govee("ihoment_H0005_1A2B"));
        assert!(is_govee("Minger_H0006_0001"));
        assert!(is_govee("Govee_H0003_ABCD"));
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
    fn the_address_is_carried_but_is_not_the_identity() {
        let seen = Advertised::read("AA:BB:CC:DD:EE:FF", "GBK_H0004_6BAF").expect("a device");
        assert_eq!(seen.address, "AA:BB:CC:DD:EE:FF");
        assert_eq!(seen.sku, "H0004");
        assert_eq!(seen.name, "GBK_H0004_6BAF");
    }
}
