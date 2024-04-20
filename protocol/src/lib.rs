use quake3::net::chan::FRAGMENT_BIT;
use std::ffi::c_int;

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct PacketSequenceNumber(c_int);

impl PacketSequenceNumber {
    // this should probably fail if -1 or FRAGMENT_BIT
    pub fn new(bits: c_int) -> Self {
        Self(bits)
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct PacketSequence(c_int);

impl PacketSequence {
    pub fn new(bits: c_int) -> Self {
        Self(bits)
    }

    pub fn new_with_number_and_fragment(number: PacketSequenceNumber, fragmented: bool) -> Self {
        if fragmented {
            Self(number.0 | FRAGMENT_BIT)
        } else {
            Self(number.0 & !FRAGMENT_BIT)
        }
    }

    pub fn is_fragmented(&self) -> bool {
        self.0 & FRAGMENT_BIT != 0
    }

    pub fn number(&self) -> PacketSequenceNumber {
        PacketSequenceNumber(self.0 & !FRAGMENT_BIT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packetsequence_new_with_number_and_fragment() {
        let sequence =
            PacketSequence::new_with_number_and_fragment(PacketSequenceNumber::new(42), true);
        assert!(sequence.is_fragmented());
        assert_eq!(sequence.number(), PacketSequenceNumber::new(42));

        let sequence =
            PacketSequence::new_with_number_and_fragment(PacketSequenceNumber::new(69), false);
        assert!(!sequence.is_fragmented());
        assert_eq!(sequence.number(), PacketSequenceNumber::new(69));
    }
}
