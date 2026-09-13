//! Finding devices: which advertised names are ours, and what one carries.
//!
//! The SKU is written into the advertised name, under one of two
//! grammars: between two underscores, or right after a `GV` prefix and before
//! four hex digits. A family advertising under another name is not found — see
//! [`Transport::bind`](super::transport::Transport::bind).
//!
//! `GBK_` and `GV` were both observed (`docs/protocol/ble.md` 1.3); the older
//! brand prefixes are reported rather than seen.
//!
//! The advertisement data of the same advertisement says whether the device
//! takes plaintext frames — see [`Beacon`] and `docs/protocol/ble.md` 1.5.

use crate::ble::wire::Heard;

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
    /// The platform decides its shape: a Bluetooth address on one, a per-host
    /// identifier on another. The adapter gets it back as it came.
    pub endpoint: String,
    /// The name it advertises.
    pub name: String,
    /// The SKU read out of that name.
    pub sku: String,
    /// What the advertisement data says, or `None` if the advertisement
    /// carries none this transport reads.
    pub beacon: Option<Beacon>,
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
            beacon: None,
        })
    }

    /// Read everything an adapter heard: the name, and the advertisement
    /// data.
    #[must_use]
    pub fn heard(heard: &Heard) -> Option<Self> {
        let mut read = Self::read(heard.endpoint.clone(), &heard.name)?;
        read.beacon = Beacon::read(&heard.adverts);
        Some(read)
    }
}

/// The advertisement data this transport reads, `docs/protocol/ble.md` 1.5.
///
/// Six bytes: a flags byte, the signature `88 EC`, a `pactType` on two bytes
/// and a `pactCode` on one. Bit `0x40` of the flags says the device encodes
/// every frame — see [`encode`](super::encode).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Beacon {
    /// Whether the device takes encoded frames only.
    pub encoded: bool,
    /// The low nibble of the flags byte: a version of the layout. Nothing
    /// here depends on it.
    pub version: u8,
    /// The `pactType` field, big-endian.
    pub pact_type: u16,
    /// The `pactCode` field.
    pub pact_code: u8,
}

/// The two bytes that follow the flags byte.
const SIGNATURE: [u8; 2] = [0x88, 0xEC];

/// The bit of the flags byte that says the device takes encoded frames only.
const ENCODED: u8 = 0x40;

impl Beacon {
    /// Read the advertisement data an adapter reports.
    ///
    /// A platform reads the first two bytes of each entry as a 16-bit prefix,
    /// little-endian, and hands the rest over. This layout does not start
    /// with such a prefix, so the two are put back together before parsing.
    /// `None` if no entry carries the layout.
    #[must_use]
    pub fn read(adverts: &[(u16, Vec<u8>)]) -> Option<Self> {
        adverts.iter().find_map(|(prefix, data)| {
            let mut raw = Vec::with_capacity(data.len() + 2);
            raw.extend_from_slice(&prefix.to_le_bytes());
            raw.extend_from_slice(data);
            Self::parse(&raw)
        })
    }

    /// Read the layout off the raw bytes of one advertisement entry.
    #[must_use]
    pub fn parse(raw: &[u8]) -> Option<Self> {
        let (&flags, rest) = raw.split_first()?;
        if rest.get(..2)? != SIGNATURE {
            return None;
        }
        let pact_type = u16::from_be_bytes([*rest.get(2)?, *rest.get(3)?]);
        Some(Self {
            encoded: flags & ENCODED != 0,
            version: flags & 0x0F,
            pact_type,
            pact_code: *rest.get(4)?,
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

    /// One bulb's advertisement, as a host reported it: the first two bytes
    /// taken for a 16-bit prefix, and the payload repeated because the scan
    /// response carries the same bytes.
    #[test]
    fn reads_the_encoding_flag_off_the_advertisement_data() {
        let heard = Heard {
            endpoint: "peripheral-1".to_owned(),
            name: "GVH600812E5".to_owned(),
            adverts: vec![(
                0x8843,
                vec![
                    0xec, 0x00, 0x01, 0x01, 0x01, 0x00, 0xec, 0x00, 0x01, 0x01, 0x01, 0x00,
                ],
            )],
        };
        let seen = Advertised::heard(&heard).expect("a device");
        assert_eq!(
            seen.beacon,
            Some(Beacon {
                encoded: true,
                version: 3,
                pact_type: 1,
                pact_code: 1,
            })
        );
    }

    #[test]
    fn a_flags_byte_without_the_bit_is_plaintext() {
        let beacon = Beacon::parse(&[0x03, 0x88, 0xec, 0x00, 0x02, 0x05]).expect("the layout");
        assert!(!beacon.encoded);
        assert_eq!(beacon.pact_type, 2);
        assert_eq!(beacon.pact_code, 5);
    }

    #[test]
    fn advertisement_data_of_another_layout_is_no_beacon() {
        assert_eq!(Beacon::read(&[(0x004c, vec![0x10, 0x05, 0x01])]), None);
        assert_eq!(Beacon::parse(&[0x43, 0x88]), None);
        assert_eq!(Beacon::read(&[]), None);
        let heard = Heard {
            endpoint: "p".to_owned(),
            name: "GBK_H0004_6BAF".to_owned(),
            adverts: Vec::new(),
        };
        assert_eq!(Advertised::heard(&heard).expect("a device").beacon, None);
    }

    #[test]
    fn the_endpoint_is_carried_but_is_not_the_identity() {
        let seen = Advertised::read("AA:BB:CC:DD:EE:FF", "GBK_H0004_6BAF").expect("a device");
        assert_eq!(seen.endpoint, "AA:BB:CC:DD:EE:FF");
        assert_eq!(seen.sku, "H0004");
        assert_eq!(seen.name, "GBK_H0004_6BAF");
    }
}
