use super::{
    ConnectionlessMessage, FragmentInfo, FragmentLength, FragmentStart,
    InvalidConnectionlessMessageError, InvalidFragmentLengthError, InvalidFragmentStartError,
    PacketKind, PacketSequenceNumber, QPort,
};
use crate::net::chan::FRAGMENT_SIZE;
use bytes::{Buf, Bytes};

#[derive(thiserror::Error, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[error("is invalid")]
pub struct InvalidSequencedMessageError {
    payload: Bytes,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct SequencedMessage {
    sequence: PacketSequenceNumber,
    qport: QPort,
    // TODO: ioq3 has additional checksum
    payload: Bytes,
}

impl SequencedMessage {
    // TODO: new_unchecked to create oversize message?
    pub fn new<T: Into<Bytes>>(
        sequence: PacketSequenceNumber,
        qport: QPort,
        payload: T,
    ) -> Result<Self, InvalidSequencedMessageError> {
        let payload: Bytes = payload.into();
        if payload.len() >= FRAGMENT_SIZE {
            Err(InvalidSequencedMessageError { payload })
        } else {
            Ok(Self {
                sequence,
                qport,
                payload,
            })
        }
    }

    pub fn sequence(&self) -> PacketSequenceNumber {
        self.sequence
    }

    pub fn qport(&self) -> QPort {
        self.qport
    }

    pub fn payload(&self) -> &Bytes {
        &self.payload
    }
}

#[derive(thiserror::Error, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[error("is invalid")]
pub struct InvalidFragmentedMessageError {
    payload: Bytes,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct FragmentedMessage {
    sequence: PacketSequenceNumber,
    qport: QPort,
    // TODO: ioq3 has additional checksum
    fragment_info: FragmentInfo,
    payload: Bytes,
}

impl FragmentedMessage {
    // TODO: new_unchecked to create oversize message?
    pub fn new<T: Into<Bytes>>(
        sequence: PacketSequenceNumber,
        qport: QPort,
        fragment_start: FragmentStart,
        payload: T,
    ) -> Result<Self, InvalidFragmentedMessageError> {
        let payload: Bytes = payload.into();
        let fragment_length = payload.len().try_into();
        match fragment_length {
            Err(_) => Err(InvalidFragmentedMessageError { payload }),
            Ok(fragment_length) => Ok(Self {
                sequence,
                qport,
                fragment_info: FragmentInfo::new(fragment_start, fragment_length),
                payload,
            }),
        }
    }

    pub fn sequence(&self) -> PacketSequenceNumber {
        self.sequence
    }

    pub fn qport(&self) -> QPort {
        self.qport
    }

    pub fn fragment_info(&self) -> FragmentInfo {
        self.fragment_info
    }

    pub fn is_last(&self) -> bool {
        self.fragment_info.is_last()
    }

    pub fn payload(&self) -> &Bytes {
        &self.payload
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum ServerMessage {
    Connectionless(ConnectionlessMessage),
    Sequenced(crate::server::SequencedMessage),
    Fragmented(crate::server::FragmentedMessage),
}

#[derive(thiserror::Error, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[error("is invalid")]
pub enum InvalidServerMessageError {
    InvalidConnectionlessMessage(#[from] InvalidConnectionlessMessageError),
    InvalidSequencedMessage(#[from] crate::server::InvalidSequencedMessageError),
    InvalidFragmentStart(#[from] InvalidFragmentStartError),
    InvalidFragmentLength(#[from] InvalidFragmentLengthError),
    InvalidFragmentedMessage(#[from] crate::server::InvalidFragmentedMessageError),
}

pub fn parse_server_packet(
    mut payload: impl Buf,
) -> Result<ServerMessage, InvalidServerMessageError> {
    // FIXME: this panics if payload doesn't have a next i32, unlike e.g. nom::Err::Incomplete
    let packet_kind = PacketKind::parse(payload.get_i32_le());

    let message = match packet_kind {
        PacketKind::Connectionless => {
            let payload = payload.copy_to_bytes(payload.remaining());
            let message = ConnectionlessMessage::new(payload)?;
            ServerMessage::Connectionless(message)
        }
        PacketKind::Sequenced(sequence) => {
            if sequence.is_fragmented() {
                // FIXME: this panics if payload doesn't have a next u16, unlike e.g. nom::Err::Incomplete
                let fragment_start = FragmentStart::new(payload.get_u16_le())?;
                // FIXME: this panics if payload doesn't have a next u16, unlike e.g. nom::Err::Incomplete
                let fragment_length = FragmentLength::new(payload.get_u16_le())?;
                let fragment_info = FragmentInfo::new(fragment_start, fragment_length);
                // TODO: this should be an error, not a panic
                assert_eq!(usize::from(fragment_info.length()), payload.remaining());
                let payload = payload.copy_to_bytes(payload.remaining());
                let message = crate::server::FragmentedMessage::new(
                    sequence.number(),
                    fragment_info.start(),
                    payload,
                )?;
                ServerMessage::Fragmented(message)
            } else {
                let payload = payload.copy_to_bytes(payload.remaining());
                let message = crate::server::SequencedMessage::new(sequence.number(), payload)?;
                ServerMessage::Sequenced(message)
            }
        }
    };

    Ok(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequencedmessage_new() -> Result<(), Box<dyn std::error::Error>> {
        assert!(SequencedMessage::new(
            PacketSequenceNumber::new(42)?,
            QPort::new(27960)?,
            vec![0; FRAGMENT_SIZE]
        )
        .is_err());

        assert!(SequencedMessage::new(
            PacketSequenceNumber::new(42)?,
            QPort::new(27960)?,
            vec![0; FRAGMENT_SIZE - 1]
        )
        .is_ok());

        Ok(())
    }

    #[test]
    fn fragmentedmessage_new() -> Result<(), Box<dyn std::error::Error>> {
        assert!(FragmentedMessage::new(
            PacketSequenceNumber::new(42)?,
            QPort::new(27960)?,
            FragmentStart::new(42)?,
            vec![0; FRAGMENT_SIZE + 1]
        )
        .is_err());

        assert!(FragmentedMessage::new(
            PacketSequenceNumber::new(42)?,
            QPort::new(27960)?,
            FragmentStart::new(42)?,
            vec![0; FRAGMENT_SIZE]
        )
        .is_ok());

        Ok(())
    }

    #[test]
    fn parse_server_packet_connectionless() -> Result<(), Box<dyn std::error::Error>> {
        let mut payload = &b"\xFF\xFF\xFF\xFF\xDE\xAD\xBE\xEF"[..];

        let message = parse_server_packet(&mut payload)?;
        match message {
            ServerMessage::Connectionless(message) => {
                assert_eq!(message.payload(), &b"\xDE\xAD\xBE\xEF"[..]);
            }
            _ => panic!(),
        }

        Ok(())
    }

    #[test]
    fn parse_server_packet_sequenced() -> Result<(), Box<dyn std::error::Error>> {
        let mut payload = &b"\x00\x00\x00\x00\xDE\xAD\xBE\xEF"[..];

        let message = parse_server_packet(&mut payload)?;
        match message {
            ServerMessage::Sequenced(message) => {
                assert_eq!(message.sequence(), PacketSequenceNumber::new(0)?);
                assert_eq!(message.payload(), &b"\xDE\xAD\xBE\xEF"[..]);
            }
            _ => panic!(),
        }

        Ok(())
    }

    #[test]
    fn parse_server_packet_fragmented() -> Result<(), Box<dyn std::error::Error>> {
        let mut payload = &b"\x00\x00\x00\x80\x01\x00\x04\x00\xDE\xAD\xBE\xEF"[..];

        let message = parse_server_packet(&mut payload)?;
        match message {
            ServerMessage::Fragmented(message) => {
                assert_eq!(message.sequence(), PacketSequenceNumber::new(0)?);
                assert_eq!(
                    message.fragment_info(),
                    FragmentInfo::new(FragmentStart::new(1)?, FragmentLength::new(4)?)
                );
                assert_eq!(message.payload(), &b"\xDE\xAD\xBE\xEF"[..]);
            }
            _ => panic!(),
        }

        Ok(())
    }
}
