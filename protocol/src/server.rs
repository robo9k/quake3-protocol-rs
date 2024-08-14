//! Packets and messages for game server
//!
//! A game server (or just server), is where game clients (or just clients) connect to.
//! Public game servers list themselves on master servers.
//! Some game servers check their clients using the auth server.
//!
//! Packets from client to server are either connectionless, sequenced or fragmented in [`ClientMessage`]:
//! - [`ConnectionlessMessage`]
//! - [`SequencedMessage`]
//! - [`FragmentedMessage`]
//!
//! Packets from master and auth server are always connectionless.
//!
//! A connectionless outer packet contains an inner message of [`ConnectionlessClientMessage`]:
//! - TODO: `GetStatusMessage`
//! - TODO: `GetInfoMessage`
//! - TODO: `GetChallengeMessage`
//! - [`ConnectMessage`]
//! - TODO:  `IpAuthorizeMessage`

pub use super::ConnectionlessMessage;

use super::{
    FragmentInfo, FragmentLength, FragmentStart, InvalidConnectionlessMessageError,
    InvalidFragmentLengthError, InvalidFragmentStartError, InvalidQPortError, PacketKind,
    PacketSequenceNumber, QPort,
};
use crate::net::chan::FRAGMENT_SIZE;
use bytes::BytesMut;
use bytes::{Buf, Bytes};
use quake3::info::InfoMap;
use quake3::info::InfoString;
use quake3::info::INFO_LIMIT;
use winnow::binary::le_u16;
use winnow::combinator::delimited;
use winnow::combinator::rest;
use winnow::combinator::seq;
use winnow::error::ContextError;
use winnow::token::literal;
use winnow::token::take_until;
use winnow::PResult;
use winnow::Parser;

/// Error for invalid [`SequencedMessage`]
#[derive(thiserror::Error, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[error("is invalid")]
pub struct InvalidSequencedMessageError {
    payload: Bytes,
}

/// Sequenced client message
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct SequencedMessage {
    sequence: PacketSequenceNumber,
    // TODO: ioq3 has additional checksum
    payload: Bytes,
}

impl SequencedMessage {
    // TODO: new_unchecked to create oversize message?
    pub fn new<T: Into<Bytes>>(
        sequence: PacketSequenceNumber,
        payload: T,
    ) -> Result<Self, InvalidSequencedMessageError> {
        let payload: Bytes = payload.into();
        if payload.len() >= FRAGMENT_SIZE {
            Err(InvalidSequencedMessageError { payload })
        } else {
            Ok(Self { sequence, payload })
        }
    }

    pub fn sequence(&self) -> PacketSequenceNumber {
        self.sequence
    }

    pub fn payload(&self) -> &Bytes {
        &self.payload
    }
}

/// Error for invalid [`FragmentedMessage`]
#[derive(thiserror::Error, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[error("is invalid")]
pub struct InvalidFragmentedMessageError {
    payload: Bytes,
}

/// Fragmented client message
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct FragmentedMessage {
    sequence: PacketSequenceNumber,
    // TODO: ioq3 has additional checksum
    fragment_info: FragmentInfo,
    payload: Bytes,
}

impl FragmentedMessage {
    // TODO: new_unchecked to create oversize message?
    pub fn new<T: Into<Bytes>>(
        sequence: PacketSequenceNumber,
        fragment_start: FragmentStart,
        payload: T,
    ) -> Result<Self, InvalidFragmentedMessageError> {
        let payload: Bytes = payload.into();
        let fragment_length = payload.len().try_into();
        match fragment_length {
            Err(_) => Err(InvalidFragmentedMessageError { payload }),
            Ok(fragment_length) => Ok(Self {
                sequence,
                fragment_info: FragmentInfo::new(fragment_start, fragment_length),
                payload,
            }),
        }
    }

    pub fn sequence(&self) -> PacketSequenceNumber {
        self.sequence
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

// FIXME: this is also kinda used for connectionless master and auth server packets
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum ClientMessage {
    Connectionless(ConnectionlessMessage),
    Sequenced(crate::client::SequencedMessage),
    Fragmented(crate::client::FragmentedMessage),
}

/// Parse error for [`ClientMessage`]
#[derive(thiserror::Error, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[error("is invalid")]
pub enum InvalidClientMessageError {
    InvalidConnectionlessMessage(#[from] InvalidConnectionlessMessageError),
    InvalidSequencedMessage(#[from] crate::client::InvalidSequencedMessageError),
    InvalidQPort(#[from] InvalidQPortError),
    InvalidFragmentStart(#[from] InvalidFragmentStartError),
    InvalidFragmentLength(#[from] InvalidFragmentLengthError),
    InvalidFragmentedMessage(#[from] crate::client::InvalidFragmentedMessageError),
}

// FIXME: this is also used for master and auth server packets
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
                // FIXME: this panics if payload doesn't have a next u16, unlike e.g. nom::Err::Incomplete
                let fragment_start = FragmentStart::new(payload.get_u16_le())?;
                // FIXME: this panics if payload doesn't have a next u16, unlike e.g. nom::Err::Incomplete
                let fragment_length = FragmentLength::new(payload.get_u16_le())?;
                let fragment_info = FragmentInfo::new(fragment_start, fragment_length);
                // TODO: this should be an error, not a panic
                assert_eq!(usize::from(fragment_info.length()), payload.remaining());
                let payload = payload.copy_to_bytes(payload.remaining());
                let message = crate::client::FragmentedMessage::new(
                    sequence.number(),
                    qport,
                    fragment_info.start(),
                    payload,
                )?;
                ClientMessage::Fragmented(message)
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

/// Kind of [`ConnectionlessClientMessage`]
pub enum ConnectionlessCommand {
    GetStatus,
    GetInfo,
    GetChallenge,
    Connect,
    IpAuthorize,
}

/// `getstatus` [`ConnectionlessCommand`]
pub const GETSTATUS_COMMAND: &[u8] = b"getstatus";
/// `getinfo` [`ConnectionlessCommand`]
pub const GETINFO_COMMAND: &[u8] = b"getinfo";
/// `getchallenge` [`ConnectionlessCommand`]
pub const GETCHALLENGE_COMMAND: &[u8] = b"getchallenge";
/// `connect` [`ConnectionlessCommand`]
pub const CONNECT_COMMAND: &[u8] = b"connect";
/// `ipAuthorize` [`ConnectionlessCommand`]
pub const IPAUTHORIZE_COMMAND: &[u8] = b"ipAuthorize";

/// Parse error for [`ConnectionlessCommand`]
#[derive(thiserror::Error, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[error("is invalid")]
pub struct ParseCommandError(());

impl ConnectionlessCommand {
    pub fn parse(bytes: &[u8]) -> Result<Self, ParseCommandError> {
        // this is case insensitive in SV_ConnectionlessPacket
        match bytes {
            GETSTATUS_COMMAND => Ok(Self::GetStatus),
            GETINFO_COMMAND => Ok(Self::GetInfo),
            GETCHALLENGE_COMMAND => Ok(Self::GetChallenge),
            CONNECT_COMMAND => Ok(Self::Connect),
            IPAUTHORIZE_COMMAND => Ok(Self::IpAuthorize),

            _ => Err(ParseCommandError(())),
        }
    }
}

/// Connectionless `connect` client message
//#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct ConnectMessage<KV> {
    user_info: InfoMap<KV, KV, { INFO_LIMIT }>,
}

/// Parse error for [`ConnectMessage`]
#[derive(thiserror::Error, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[error("could not parse")]
pub struct ParseConnectMessageError(());

fn recognize_connect_payload<'s>() -> impl Parser<&'s [u8], &'s [u8], ContextError> {
    literal(CONNECT_COMMAND)
}

fn parse_connect_payload(input: &mut &[u8]) -> PResult<ConnectMessage<InfoString>> {
    // 0. "connect" in recognize_connect_payload()
    // 1. " " (space)
    // 2. u16 decoded huffman len, huffman blob
    // 3. decoded blob: \" .. user_info .. \"

    // Q3 peeks the "connect", then overwrites the original msg buffer with the huffman decoded part
    // i.e. it ends up with a complete string buffer of: connect "<user_info>"
    // could be emulated with https://docs.rs/bytes/latest/bytes/buf/struct.Chain.html but likely not needed

    // MSG_ReadStringLine(), Cmd_TokenizeString() probably overkill for MVP
    // see https://github.com/robo9k/quake3-file-parsers/blob/main/src/lexer.rs

    let (len, bytes) = seq!(
        _: literal(b" "),
        le_u16,
        rest,
    )
    .parse_next(input)?;

    let mut huff = quake3_huffman::Huffman::adaptive();
    let mut decoded = BytesMut::new();

    huff.decode(&bytes[..], len.into(), &mut decoded).unwrap();

    let user_info =
        delimited(b"\"", take_until(1.., b'\"'), b"\"").parse_next(&mut &decoded[..])?;

    let user_info = InfoMap::<InfoString, InfoString, INFO_LIMIT>::parse(user_info).unwrap();

    let connect_message = ConnectMessage::new(user_info);
    Ok(connect_message)
}

impl<KV> ConnectMessage<KV> {
    pub fn new(user_info: InfoMap<KV, KV, { INFO_LIMIT }>) -> Self {
        Self { user_info }
    }

    pub fn user_info(&self) -> &InfoMap<KV, KV, { INFO_LIMIT }> {
        &self.user_info
    }

    pub fn parse_message(
        message: &ConnectionlessMessage,
    ) -> Result<ConnectMessage<InfoString>, ParseConnectMessageError> {
        let payload = message.payload();
        let mut payload = &payload.as_ref();
        let (connect_message,) = seq!(_: recognize_connect_payload(), parse_connect_payload)
            .parse(payload)
            .map_err(|_e| ParseConnectMessageError(()))?;
        Ok(connect_message)
    }
}

// FIXME: this name is confusing, maybe have outer Packet vs. inner Message in all the modules?
/// Kind of connectionless [`ClientMessage`]
pub enum ConnectionlessClientMessage {
    GetStatus(()),
    GetInfo(()),
    GetChallenge(()),
    Connect(ConnectMessage<InfoString>), // that <KV> generic is annoying here, maybe less so if this were OwnedConnectionlessClientMessage ?
    IpAuthorize(()),
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
                assert_eq!(message.payload(), &b"\xDE\xAD\xBE\xEF"[..]);
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
                assert_eq!(message.payload(), &b"\xDE\xAD\xBE\xEF"[..]);
            }
            _ => panic!(),
        }

        Ok(())
    }

    #[test]
    fn parse_client_packet_fragmented() -> Result<(), Box<dyn std::error::Error>> {
        let mut payload = &b"\x00\x00\x00\x80\x9A\x02\x01\x00\x04\x00\xDE\xAD\xBE\xEF"[..];

        let message = parse_client_packet(&mut payload)?;
        match message {
            ClientMessage::Fragmented(message) => {
                assert_eq!(message.sequence(), PacketSequenceNumber::new(0)?);
                assert_eq!(message.qport(), QPort::new(666)?);
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

    #[test]
    fn connectmessage_parse_message() -> Result<(), Box<dyn std::error::Error>> {
        const encoded_bytes: [u8; 239] = hex_literal::hex!(
            "
            63 6F 6E 6E 65 63 74 20
            28 01
            44 74 30 8e 05 0c c7 26 
            c3 14 ec 8e f9 67 d0 1a 4e 29 98 01 c7 c3 7a 30 
            2c 2c 19 1c 13 87 c2 de 71 0a 5c ac 30 cd 40 ce 
            3a ca af 96 2a b0 d9 3a b7 b0 fd 4d a8 0e c9 ba 
            79 4c 28 0a c4 0a 4f 83 02 9b 9f 69 e4 0a c3 38 
            47 9b cf 22 af 61 f6 64 6f 13 7c a3 ae 1f af 06 
            52 b7 3c a3 06 5f 3a f4 8f 66 d2 40 ac ee 2b 2d 
            ea 38 18 f9 b7 f2 36 37 80 ea 17 e9 d5 40 58 f7 
            0f c6 b2 3a 85 e5 bb ca f7 78 77 09 2c e1 e5 7b 
            cc ad 59 0f 3c ea 67 2a 37 1a 31 c7 83 e5 02 d7 
            d1 dd c0 73 eb e6 5d 4c 32 87 a4 a4 8d 2e 1b 08 
            0b 38 11 ac 7b 9a 34 16 e2 e6 d1 3b f0 f8 f2 99 
            da c4 91 b7 4b 53 cf 82 a6 da 10 61 89 b0 5b 6c 
            6e c3 46 e3 b7 7c 19 62 38 ac 42 48 23 ab 11 e6 
            20 0a b8 75 91 26 12 6e 92 25 65 c9 00       
        "
        );

        let message = ConnectionlessMessage::new(&encoded_bytes[..])?;
        let connect_message = ConnectMessage::<InfoString>::parse_message(&message)?;

        let user_info = connect_message.user_info();

        assert!(user_info.len() == 19);

        Ok(())
    }
}
