//! The `ArtDmx` sequence field: what tells a late packet from a new one.
//!
//! UDP delivers out of order. The sender counts 1 to 255 and wraps, and the
//! bridge drops a packet that is older than the last one it accepted. A
//! sender that writes 0 disables the check — see `docs/dmx.md`.
//!
//! The gate holds one sender on one port-address. [`Gate`] is what keeps one
//! per pair.

use std::collections::HashMap;
use std::net::SocketAddr;

/// The value a sender writes to disable the check.
const DISABLED: u8 = 0;
/// Half of the 256 values. A packet this far ahead of the last one is a
/// packet that far behind it, and the split has to fall somewhere.
const HALF: u8 = 128;

/// What orders the packets of one sender on one port-address.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Sequence {
    last: Option<u8>,
}

impl Sequence {
    /// A gate that has accepted nothing.
    #[must_use]
    pub const fn new() -> Self {
        Self { last: None }
    }

    /// Whether to take the packet that carries `sequence`.
    ///
    /// The first packet is always taken. A sequence of 0 is always taken
    /// and forgets the count, so that a sender which stops counting keeps
    /// driving.
    pub fn accept(&mut self, sequence: u8) -> bool {
        if sequence == DISABLED {
            self.last = None;
            return true;
        }
        let Some(last) = self.last else {
            self.last = Some(sequence);
            return true;
        };
        if sequence.wrapping_sub(last) >= HALF {
            return false;
        }
        self.last = Some(sequence);
        true
    }

    /// The sequence of the last packet taken, and `None` where the check is
    /// off or nothing arrived.
    #[must_use]
    pub const fn last(self) -> Option<u8> {
        self.last
    }
}

/// One [`Sequence`] per sender and port-address.
///
/// Two senders on one port-address count apart, so the packets of one never
/// drop the packets of the other. Art-Net asks for an HTP merge of two
/// sources; the node takes the last one instead — see `docs/dmx.md`.
#[derive(Debug, Clone, Default)]
pub struct Gate {
    senders: HashMap<(SocketAddr, u16), Sequence>,
}

impl Gate {
    /// A gate that has accepted nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether to take the packet `source` sent on `universe` with
    /// `sequence`.
    pub fn accept(&mut self, source: SocketAddr, universe: u16, sequence: u8) -> bool {
        self.senders
            .entry((source, universe))
            .or_default()
            .accept(sequence)
    }

    /// How many sender and port-address pairs the gate holds.
    #[must_use]
    pub fn senders(&self) -> usize {
        self.senders.len()
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use super::{Gate, Sequence};

    fn address(host: u8) -> SocketAddr {
        SocketAddr::from(([192, 0, 2, host], 6454))
    }

    /// One sender's count never drops another sender's packets.
    #[test]
    fn two_senders_count_apart() {
        let mut gate = Gate::new();
        assert!(gate.accept(address(2), 0, 100));
        assert!(gate.accept(address(3), 0, 4));
        assert!(!gate.accept(address(2), 0, 99));
        assert_eq!(gate.senders(), 2);
    }

    /// One sender on two port-addresses counts each one apart.
    #[test]
    fn one_sender_counts_each_port_address_apart() {
        let mut gate = Gate::new();
        assert!(gate.accept(address(2), 0, 100));
        assert!(gate.accept(address(2), 1, 4));
        assert_eq!(gate.senders(), 2);
    }

    #[test]
    fn the_first_packet_is_taken() {
        assert!(Sequence::new().accept(7));
    }

    #[test]
    fn a_packet_that_counts_on_is_taken() {
        let mut gate = Sequence::new();
        for sequence in 1..=255u8 {
            assert!(gate.accept(sequence), "{sequence}");
        }
    }

    #[test]
    fn a_packet_older_than_the_last_one_is_dropped() {
        let mut gate = Sequence::new();
        assert!(gate.accept(100));
        assert!(!gate.accept(99));
        assert!(!gate.accept(1));
        assert_eq!(gate.last(), Some(100));
    }

    /// The count wraps at 255, so 2 is newer than 250 and 250 is older.
    #[test]
    fn the_count_wraps() {
        let mut gate = Sequence::new();
        assert!(gate.accept(250));
        assert!(gate.accept(2));
        assert!(!gate.accept(250));
    }

    /// A sender that stops counting keeps driving the rig.
    #[test]
    fn a_zero_disables_the_check() {
        let mut gate = Sequence::new();
        assert!(gate.accept(200));
        assert!(gate.accept(0));
        assert_eq!(gate.last(), None);
        assert!(gate.accept(1));
    }

    /// A desk that repeats one look repeats one sequence, and the look must
    /// still arrive.
    #[test]
    fn a_repeated_sequence_is_taken() {
        let mut gate = Sequence::new();
        assert!(gate.accept(42));
        assert!(gate.accept(42));
    }
}
