use super::{
    ConnectionlessMessage, FragmentStart, InvalidConnectionlessMessageError, PacketKind,
    PacketSequenceNumber, QPort,
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
    qport: QPort,
    // TODO: ioq3 has additional checksum
    data: Bytes,
}

impl SequencedMessage {
    // TODO: new_unchecked to create oversize message?
    pub fn new<T: Into<Bytes>>(
        sequence: PacketSequenceNumber,
        qport: QPort,
        data: T,
    ) -> Result<Self, InvalidSequencedMessageError> {
        let data: Bytes = data.into();
        if data.len() >= FRAGMENT_SIZE {
            Err(InvalidSequencedMessageError { data })
        } else {
            Ok(Self {
                sequence,
                qport,
                data,
            })
        }
    }

    pub fn sequence(&self) -> PacketSequenceNumber {
        self.sequence
    }

    pub fn qport(&self) -> QPort {
        self.qport
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
    qport: QPort,
    // TODO: ioq3 has additional checksum
    fragment_start: FragmentStart,
    data: Bytes,
}

impl FragmentedMessage {
    // TODO: new_unchecked to create oversize message?
    pub fn new<T: Into<Bytes>>(
        sequence: PacketSequenceNumber,
        qport: QPort,
        fragment_start: FragmentStart,
        data: T,
    ) -> Result<Self, InvalidFragmentedMessageError> {
        let data: Bytes = data.into();
        if data.len() > FRAGMENT_SIZE {
            Err(InvalidFragmentedMessageError { data })
        } else {
            Ok(Self {
                sequence,
                qport,
                fragment_start,
                data,
            })
        }
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
                // fragmentStart = MSG_ReadShort( payload );
                // fragmentLength = MSG_ReadShort( payload );

                // let fragment_start = FragmentStart::new(fragmentStart)?;
                // let fragment_length = FragmentLength::new(fragmentStart)?;
                // let fragment_info = FragmentInfo::new(fragment_start, fragment_length);
                // let payload = payload.copy_to_bytes(payload.remaining());
                // let message = crate::server::FragmentedMessage::new(sequence.number(), fragment_info.start(), payload);
                // ServerMessage::Fragmented(message)
                todo!();
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
                assert_eq!(message.data(), &b"\xDE\xAD\xBE\xEF"[..]);
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
                assert_eq!(message.data(), &b"\xDE\xAD\xBE\xEF"[..]);
            }
            _ => panic!(),
        }

        Ok(())
    }
}
