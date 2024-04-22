use quake3::net::chan::FRAGMENT_BIT;
use std::ffi::c_int;

const CONNECTIONLESS_SEQUENCE: c_int = 0xFF_FF_FF_FFu32 as i32;

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct PacketSequenceNumber(c_int);

impl PacketSequenceNumber {
    pub fn new(bits: c_int) -> Result<Self, InvalidPacketSequenceNumberError> {
        if CONNECTIONLESS_SEQUENCE == bits {
            Err(InvalidPacketSequenceNumberError(()))
        } else if bits & FRAGMENT_BIT != 0 {
            Err(InvalidPacketSequenceNumberError(()))
        } else {
            Ok(Self(bits))
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct InvalidPacketSequenceNumberError(());

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

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum PacketKind {
    Connectionless,
    Sequenced(PacketSequence),
}

impl PacketKind {
    pub fn parse(bits: c_int) -> Self {
        if CONNECTIONLESS_SEQUENCE == bits {
            Self::Connectionless
        } else {
            Self::Sequenced(PacketSequence::new(bits))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packetsequencenumber_new() {
        assert!(PacketSequenceNumber::new(CONNECTIONLESS_SEQUENCE).is_err());

        assert!(PacketSequenceNumber::new(42 | FRAGMENT_BIT).is_err());

        assert!(PacketSequenceNumber::new(42).is_ok());
    }

    #[test]
    fn packetsequence_new_with_number_and_fragment() -> Result<(), InvalidPacketSequenceNumberError>
    {
        let sequence =
            PacketSequence::new_with_number_and_fragment(PacketSequenceNumber::new(42)?, true);
        assert!(sequence.is_fragmented());
        assert_eq!(sequence.number(), PacketSequenceNumber::new(42)?);

        let sequence =
            PacketSequence::new_with_number_and_fragment(PacketSequenceNumber::new(69)?, false);
        assert!(!sequence.is_fragmented());
        assert_eq!(sequence.number(), PacketSequenceNumber::new(69)?);

        Ok(())
    }

    #[test]
    fn packetkind_parse() -> Result<(), InvalidPacketSequenceNumberError> {
        assert_eq!(
            PacketKind::parse(0xFF_FF_FF_FFu32 as i32),
            PacketKind::Connectionless
        );

        assert_eq!(
            PacketKind::parse(0x00_00_00_FFu32 as i32),
            PacketKind::Sequenced(PacketSequence::new_with_number_and_fragment(
                PacketSequenceNumber::new(0xFF)?,
                false
            ))
        );

        assert_eq!(
            PacketKind::parse(0x80_00_00_FFu32 as i32),
            PacketKind::Sequenced(PacketSequence::new_with_number_and_fragment(
                PacketSequenceNumber::new(0xFF)?,
                true
            ))
        );

        Ok(())
    }
}
