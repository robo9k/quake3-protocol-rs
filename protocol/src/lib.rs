use bytes::Bytes;
use quake3::net::chan::{FRAGMENT_BIT, FRAGMENT_SIZE, MAX_PACKETLEN};
use std::ffi::{c_int, c_ushort};

pub mod client;
pub mod server;

const CONNECTIONLESS_SEQUENCE: c_int = 0xFF_FF_FF_FFu32 as i32;

#[derive(thiserror::Error, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[error("is invalid")]
pub struct InvalidPacketSequenceNumberError(());

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
    // TODO: Separate Sequenced(PacketSequenceNumber), Fragmented(FragmentInfo)? But FragmentInfo requires reading more of the packet
    // ↑ requires reading qport for client (and checksum for all ioq3) packets, so whole enum would be specific to each crate::*::FragmentedMessage
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

#[derive(thiserror::Error, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[error("is invalid")]
pub struct InvalidConnectionlessMessageError {
    data: Bytes,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct ConnectionlessMessage {
    data: Bytes,
}

impl ConnectionlessMessage {
    // TODO: new_unckecked to create oversize message?
    pub fn new<T: Into<Bytes>>(data: T) -> Result<Self, InvalidConnectionlessMessageError> {
        let data: Bytes = data.into();
        if data.len() > MAX_PACKETLEN {
            Err(InvalidConnectionlessMessageError { data })
        } else {
            Ok(Self { data })
        }
    }

    pub fn data(&self) -> &Bytes {
        &self.data
    }
}

#[derive(thiserror::Error, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[error("is invalid")]
pub struct InvalidFragmentStartError(());

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct FragmentStart(c_ushort);

impl FragmentStart {
    // TODO: new_unchecked to create oversize fragment start?
    pub fn new(start: c_ushort) -> Result<Self, InvalidFragmentStartError> {
        if usize::from(start) >= MAX_PACKETLEN {
            Err(InvalidFragmentStartError(()))
        } else {
            Ok(Self(start))
        }
    }
}

#[derive(thiserror::Error, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[error("is invalid")]
pub struct InvalidFragmentLengthError(());

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct FragmentLength(c_ushort);

impl FragmentLength {
    // TODO: new_unchecked to create oversize fragment length?
    pub fn new(length: c_ushort) -> Result<Self, InvalidFragmentLengthError> {
        if usize::from(length) > FRAGMENT_SIZE {
            Err(InvalidFragmentLengthError(()))
        } else {
            Ok(Self(length))
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct FragmentInfo {
    start: FragmentStart,
    length: FragmentLength,
}

impl FragmentInfo {
    pub fn new(start: FragmentStart, length: FragmentLength) -> Self {
        Self { start, length }
    }

    pub fn start(&self) -> FragmentStart {
        self.start
    }

    pub fn length(&self) -> FragmentLength {
        self.length
    }
}

#[derive(thiserror::Error, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[error("is invalid")]
pub struct InvalidQPortError(());

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
// TODO: NonZero?
pub struct QPort(c_ushort);

impl QPort {
    // TODO: new_unchecked to create invalid qport 0?
    pub fn new(port: c_ushort) -> Result<Self, InvalidQPortError> {
        if 0 == port {
            Err(InvalidQPortError(()))
        } else {
            Ok(Self(port))
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
    fn packetsequence_new_with_number_and_fragment() -> Result<(), Box<dyn std::error::Error>> {
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
    fn packetkind_parse() -> Result<(), Box<dyn std::error::Error>> {
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

    #[test]
    fn connectionlessmessage_new() {
        assert!(ConnectionlessMessage::new(vec![0; MAX_PACKETLEN + 1]).is_err());

        assert!(ConnectionlessMessage::new(&[] as &[u8]).is_ok());

        assert!(ConnectionlessMessage::new(&[0xDE, 0xAD, 0xBE, 0xEF][..]).is_ok());
    }

    #[test]
    fn fragmentstart_new() {
        assert!(FragmentStart::new(MAX_PACKETLEN as c_ushort).is_err());

        assert!(FragmentStart::new(42).is_ok());
    }

    #[test]
    fn fragmentlength_new() {
        assert!(FragmentLength::new(FRAGMENT_SIZE as c_ushort + 1).is_err());

        assert!(FragmentLength::new(42).is_ok());
    }

    #[test]
    fn qport_new() {
        assert!(QPort::new(0).is_err());

        assert!(QPort::new(27960).is_ok());
    }
}
