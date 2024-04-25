use super::{
    ConnectionlessMessage, FragmentStart, InvalidConnectionlessMessageError, InvalidQPortError,
    PacketKind, PacketSequenceNumber, QPort,
};
use bytes::{Buf, Bytes};
use quake3::net::chan::FRAGMENT_SIZE;

#[derive(thiserror::Error, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[error("is invalid")]
pub struct InvalidSequencedMessageError {
    data: Bytes,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct SequencedMessage {
    sequence: PacketSequenceNumber,
    // TODO: ioq3 has additional checksum
    data: Bytes,
}

impl SequencedMessage {
    // TODO: new_unchecked to create oversize message?
    pub fn new<T: Into<Bytes>>(
        sequence: PacketSequenceNumber,
        data: T,
    ) -> Result<Self, InvalidSequencedMessageError> {
        let data: Bytes = data.into();
        if data.len() >= FRAGMENT_SIZE {
            Err(InvalidSequencedMessageError { data })
        } else {
            Ok(Self { sequence, data })
        }
    }

    pub fn sequence(&self) -> PacketSequenceNumber {
        self.sequence
    }

    pub fn data(&self) -> &Bytes {
        &self.data
    }
}

#[derive(thiserror::Error, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[error("is invalid")]
pub struct InvalidFragmentedMessageError {
    data: Bytes,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct FragmentedMessage {
    sequence: PacketSequenceNumber,
    // TODO: ioq3 has additional checksum
    fragment_start: FragmentStart,
    data: Bytes,
}

impl FragmentedMessage {
    // TODO: new_unchecked to create oversize message?
    pub fn new<T: Into<Bytes>>(
        sequence: PacketSequenceNumber,
        fragment_start: FragmentStart,
        data: T,
    ) -> Result<Self, InvalidFragmentedMessageError> {
        let data: Bytes = data.into();
        if data.len() > FRAGMENT_SIZE {
            Err(InvalidFragmentedMessageError { data })
        } else {
            Ok(Self {
                sequence,
                fragment_start,
                data,
            })
        }
    }

    // TODO: getter for sequence, fragment_start or FragmentInfo?, data
    // TODO: fn is_last_fragment data.len() != FRAGMENT_SIZE or in FragmentInfo
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum ClientMessage {
    Connectionless(ConnectionlessMessage),
    Sequenced(crate::client::SequencedMessage),
    Fragmented(crate::client::FragmentedMessage),
}

#[derive(thiserror::Error, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[error("is invalid")]
pub enum InvalidClientMessageError {
    InvalidConnectionlessMessage(#[from] InvalidConnectionlessMessageError),
    InvalidSequencedMessage(#[from] crate::client::InvalidSequencedMessageError),
    InvalidQPort(#[from] InvalidQPortError),
}

pub fn parse_client_packet(
    mut payload: impl Buf,
) -> Result<ClientMessage, InvalidClientMessageError> {
    // FIXME: this panics if payload doesn't have a next i32, unlike e.g. nom::Err::Incomplete
    let packet_kind = PacketKind::parse(payload.get_i32_le());

    let message = match packet_kind {
        PacketKind::Connectionless => {
            let payload = payload.copy_to_bytes(payload.remaining());
            let message = ConnectionlessMessage::new(payload)?;
            ClientMessage::Connectionless(message)
        }
        PacketKind::Sequenced(sequence) => {
            // FIXME: this panics if payload doesn't have a next u16, unlike e.g. nom::Err::Incomplete
            let qport = QPort::new(payload.get_u16_le())?;

            if sequence.is_fragmented() {
                // fragmentStart = MSG_ReadShort( payload );
                // fragmentLength = MSG_ReadShort( payload );

                // let fragment_start = FragmentStart::new(fragmentStart)?;
                // let fragment_length = FragmentLength::new(fragmentLength)?;
                // let fragment_info = FragmentInfo::new(fragment_start, fragment_length);
                // assert_eq!(fragment.length(), payload.remaining());
                // let payload = payload.copy_to_bytes(payload.remaining());
                // let message = crate::client::FragmentedMessage::new(sequence.number(), qport, fragment_info.start(), payload);
                // ClientMessage::Fragmented(message)
                todo!();
            } else {
                let payload = payload.copy_to_bytes(payload.remaining());
                let message =
                    crate::client::SequencedMessage::new(sequence.number(), qport, payload)?;
                ClientMessage::Sequenced(message)
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
        assert!(
            SequencedMessage::new(PacketSequenceNumber::new(42)?, vec![0; FRAGMENT_SIZE]).is_err()
        );

        assert!(
            SequencedMessage::new(PacketSequenceNumber::new(42)?, vec![0; FRAGMENT_SIZE - 1])
                .is_ok()
        );

        Ok(())
    }

    #[test]
    fn fragmentedmessage_new() -> Result<(), Box<dyn std::error::Error>> {
        assert!(FragmentedMessage::new(
            PacketSequenceNumber::new(42)?,
            FragmentStart::new(42)?,
            vec![0; FRAGMENT_SIZE + 1]
        )
        .is_err());

        assert!(FragmentedMessage::new(
            PacketSequenceNumber::new(42)?,
            FragmentStart::new(42)?,
            vec![0; FRAGMENT_SIZE]
        )
        .is_ok());

        Ok(())
    }

    #[test]
    fn parse_client_packet_connectionless() -> Result<(), Box<dyn std::error::Error>> {
        let mut payload = &b"\xFF\xFF\xFF\xFF\xDE\xAD\xBE\xEF"[..];

        let message = parse_client_packet(&mut payload)?;
        match message {
            ClientMessage::Connectionless(message) => {
                assert_eq!(message.data(), &b"\xDE\xAD\xBE\xEF"[..]);
            }
            _ => panic!(),
        }

        Ok(())
    }

    #[test]
    fn parse_client_packet_sequenced() -> Result<(), Box<dyn std::error::Error>> {
        let mut payload = &b"\x00\x00\x00\x00\x9A\x02\xDE\xAD\xBE\xEF"[..];

        let message = parse_client_packet(&mut payload)?;
        match message {
            ClientMessage::Sequenced(message) => {
                assert_eq!(message.sequence(), PacketSequenceNumber::new(0)?);
                assert_eq!(message.qport(), QPort::new(666)?);
                assert_eq!(message.data(), &b"\xDE\xAD\xBE\xEF"[..]);
            }
            _ => panic!(),
        }

        Ok(())
    }
}
