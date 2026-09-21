//! The `ArtDmx` sequence field: what tells a late packet from a new one — see
//! `docs/dmx.md` 5.1.
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
/// How many packets in a row the gate refuses before it takes the count as
/// restarted. A sender that jumps half the range forward reads as older for
/// every packet that follows, and the gate would refuse it for good.
const RESYNC: u8 = 4;

/// What orders the packets of one sender on one port-address.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Sequence {
    last: Option<u8>,
    /// How many packets the gate refused since the last one it took.
    refused: u8,
}

impl Sequence {
    /// A gate that has accepted nothing.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            last: None,
            refused: 0,
        }
    }

    /// Whether to take the packet that carries `sequence`.
    ///
    /// The first packet is always taken. A sequence of 0 is always taken and
    /// forgets the count, so a sender which stops counting keeps driving.
    /// After 4 refusals in a row the gate takes the packet and counts from
    /// it: a sender that restarted must reach the rig again.
    pub fn accept(&mut self, sequence: u8) -> bool {
        if sequence == DISABLED {
            self.last = None;
            self.refused = 0;
            return true;
        }
        if let Some(last) = self.last
            && sequence.wrapping_sub(last) >= HALF
            && self.refused < RESYNC
        {
            self.refused += 1;
            return false;
        }
        self.last = Some(sequence);
        self.refused = 0;
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

    #[test]
    fn two_senders_count_apart() {
        let mut gate = Gate::new();
        assert!(gate.accept(address(2), 0, 100));
        assert!(gate.accept(address(3), 0, 4));
        assert!(!gate.accept(address(2), 0, 99));
        assert_eq!(gate.senders(), 2);
    }

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

    /// A rig that refuses every packet of the one desk on the network goes
    /// dark.
    #[test]
    fn a_sender_that_counts_from_somewhere_else_is_taken_back() {
        let mut gate = Sequence::new();
        assert!(gate.accept(10));
        for sequence in 200..204u8 {
            assert!(!gate.accept(sequence), "{sequence}");
        }
        assert!(gate.accept(204));
        assert_eq!(gate.last(), Some(204));
    }

    /// The refusals that resync have to be in a row.
    #[test]
    fn one_late_packet_does_not_count_towards_a_resync() {
        let mut gate = Sequence::new();
        assert!(gate.accept(100));
        for sequence in 101..=120u8 {
            assert!(!gate.accept(1), "{sequence}");
            assert!(gate.accept(sequence), "{sequence}");
            assert_eq!(gate.last(), Some(sequence));
        }
    }

    #[test]
    fn the_count_wraps() {
        let mut gate = Sequence::new();
        assert!(gate.accept(250));
        assert!(gate.accept(2));
        assert!(!gate.accept(250));
    }

    #[test]
    fn a_zero_disables_the_check() {
        let mut gate = Sequence::new();
        assert!(gate.accept(200));
        assert!(gate.accept(0));
        assert_eq!(gate.last(), None);
        assert!(gate.accept(1));
    }

    /// A desk that repeats one look repeats one sequence.
    #[test]
    fn a_repeated_sequence_is_taken() {
        let mut gate = Sequence::new();
        assert!(gate.accept(42));
        assert!(gate.accept(42));
    }
}
