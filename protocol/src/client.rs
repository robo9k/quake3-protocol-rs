use super::{FragmentStart, PacketSequenceNumber, QPort};
use bytes::Bytes;
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
}
