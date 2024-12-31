//! Packets and messages for game servers
//!
//! A game server is where game clients connect to.
//! Public game servers list themselves on master servers.
//! Some game servers check their game clients using the auth server.
//!
//! Incoming packets from game clients are either connectionless, sequenced or fragmented in [`Packet`].
//! Outgoing packets to game clients are either:
//! - [`ConnectionlessPacket`]
//! - [`SequencedPacket`]
//! - [`FragmentedPacket`]
//!
//! Packets from and to master servers and auth server are always connectionless.
//!
//! A connectionless outer packet contains an inner message of [`ConnectionlessMessage`]:
//! - TODO: `GetStatusMessage`
//! - TODO: `GetInfoMessage`
//! - TODO: `GetChallengeMessage`
//! - [`ConnectMessage`]
//! - TODO:  `IpAuthorizeMessage`

pub use super::ConnectionlessPacket;

use super::{
    FragmentInfo, FragmentLength, FragmentStart, InvalidConnectionlessPacketError,
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

/// Error for invalid [`SequencedPacket`]
#[derive(thiserror::Error, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[error("is invalid")]
pub struct InvalidSequencedPacketError {
    payload: Bytes,
}

/// Sequenced outgoing client packet
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct SequencedPacket {
    sequence: PacketSequenceNumber,
    // TODO: ioq3 has additional checksum
    payload: Bytes,
}

impl SequencedPacket {
    // TODO: new_unchecked to create oversize packet?
    pub fn new<T: Into<Bytes>>(
        sequence: PacketSequenceNumber,
        payload: T,
    ) -> Result<Self, InvalidSequencedPacketError> {
        let payload: Bytes = payload.into();
        if payload.len() >= FRAGMENT_SIZE {
            Err(InvalidSequencedPacketError { payload })
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

/// Error for invalid [`FragmentedPacket`]
#[derive(thiserror::Error, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[error("is invalid")]
pub struct InvalidFragmentedPacketError {
    payload: Bytes,
}

/// Fragmented outgoing client packet
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct FragmentedPacket {
    sequence: PacketSequenceNumber,
    // TODO: ioq3 has additional checksum
    fragment_info: FragmentInfo,
    payload: Bytes,
}

impl FragmentedPacket {
    // TODO: new_unchecked to create oversize packet?
    pub fn new<T: Into<Bytes>>(
        sequence: PacketSequenceNumber,
        fragment_start: FragmentStart,
        payload: T,
    ) -> Result<Self, InvalidFragmentedPacketError> {
        let payload: Bytes = payload.into();
        let fragment_length = payload.len().try_into();
        match fragment_length {
            Err(_) => Err(InvalidFragmentedPacketError { payload }),
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

/// Incoming packet
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Packet {
    Connectionless(ConnectionlessPacket),
    Sequenced(crate::client::SequencedPacket),
    Fragmented(crate::client::FragmentedPacket),
}

/// Parse error for [`Packet`]
#[derive(thiserror::Error, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[error("is invalid")]
pub enum InvalidPacketError {
    InvalidConnectionlessPacket(#[from] InvalidConnectionlessPacketError),
    InvalidSequencedPacket(#[from] crate::client::InvalidSequencedPacketError),
    InvalidQPort(#[from] InvalidQPortError),
    InvalidFragmentStart(#[from] InvalidFragmentStartError),
    InvalidFragmentLength(#[from] InvalidFragmentLengthError),
    InvalidFragmentedPacket(#[from] crate::client::InvalidFragmentedPacketError),

    InvalidSize,
}

// TODO: this function starts parsing things the user might not need nor want (performance, security)
// specifically if implementing a "master client" (net/client.rs), we do not need any PacketKind::Sequenced
// we should probably have something like: peek_packet(Buf) -> (PacketKind, Fn() -> Result<Packet, InvalidPacketError>)
// see https://rust-lang.github.io/api-guidelines/flexibility.html#c-intermediate but keep winnow out of our public API
// parts of the crate should also be fatures, e.g. "master client" which means some structs are missing and parsing returns "this is unsupported (but known)" errors
// the closure IF called will continue parsing the remaining buffer into e.g. PacketKind::Fragmented / Packet::Sequenced
// the closure avoids the user handling the buffer and calling pub methods for partially parsed inputs
// this could be used for the next onion layer of peeking the command kind (first lexed token), then CONDITIONALLY parsing the remaining command message / tokens, i.e.
// fn peek_command(ConnectionlessPacket) -> (ConnectionlessCommandKind, Fn() -> Result<ConnectionlessCommand, InvalidCommandError>)
// for the sequenced packets that closure probably needs to take some (mutable?) TBD client/server netchan state (challenge, sequence +/ server id, last command)
// as input to xor unscamble/decode idq3 and checksum ioq3
/// Parse incoming packet
pub fn parse_packet(mut payload: impl Buf) -> Result<Packet, InvalidPacketError> {
    // the bytes crate would be nicer with fallible try_get_* methods https://github.com/tokio-rs/bytes/issues/254
    if payload.remaining() < core::mem::size_of::<i32>() {
        return Err(InvalidPacketError::InvalidSize);
    }
    let packet_kind = PacketKind::parse(payload.get_i32_le());

    let packet = match packet_kind {
        PacketKind::Connectionless => {
            let payload = payload.copy_to_bytes(payload.remaining());
            let packet = ConnectionlessPacket::new(payload)?;
            Packet::Connectionless(packet)
        }
        PacketKind::Sequenced(sequence) => {
            if payload.remaining() < core::mem::size_of::<u16>() {
                return Err(InvalidPacketError::InvalidSize);
            }
            let qport = QPort::new(payload.get_u16_le())?;

            if sequence.is_fragmented() {
                if payload.remaining() < core::mem::size_of::<u16>() {
                    return Err(InvalidPacketError::InvalidSize);
                }
                let fragment_start = FragmentStart::new(payload.get_u16_le())?;

                if payload.remaining() < core::mem::size_of::<u16>() {
                    return Err(InvalidPacketError::InvalidSize);
                }
                let fragment_length = FragmentLength::new(payload.get_u16_le())?;

                let fragment_info = FragmentInfo::new(fragment_start, fragment_length);
                // TODO: this should be an error, not a panic
                assert_eq!(usize::from(fragment_info.length()), payload.remaining());

                let payload = payload.copy_to_bytes(payload.remaining());
                let packet = crate::client::FragmentedPacket::new(
                    sequence.number(),
                    qport,
                    fragment_info.start(),
                    payload,
                )?;
                Packet::Fragmented(packet)
            } else {
                let payload = payload.copy_to_bytes(payload.remaining());
                let packet =
                    crate::client::SequencedPacket::new(sequence.number(), qport, payload)?;
                Packet::Sequenced(packet)
            }
        }
    };

    Ok(packet)
}

/// Kind of incoming [`ConnectionlessMessage`]
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

/// Connectionless incoming `connect` client message
// TODO: Expose intermediate CompressedConnectMessage for fuzzing and zip-bomb defusal?
//#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct ConnectMessage<KV> {
    // TODO: UserInfo struct with parsed
    // - protocol
    // - challenge
    // - qport
    // - rate
    // - snaps
    // - cl_anonymous
    // - cl_voipProtocol
    // - cl_guid
    // - password
    //
    // and possibly more. Or split into engine/qagame (id/io)?
    // At least `protocol` will be required for later id/io split parsers and structs
    // `challenge` and `protocol` are in `challengeResponse` server → client message
    // `qport` is in sequenced client → server packets
    //
    // Also note that the client connect userinfo differs from the calculated server userinfo, e.g. with "ip":"localhost"
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

    pub fn parse_packet(
        packet: &ConnectionlessPacket,
    ) -> Result<ConnectMessage<InfoString>, ParseConnectMessageError> {
        let payload = packet.payload();
        let mut payload = &payload.as_ref();
        let (connect_message,) = seq!(_: recognize_connect_payload(), parse_connect_payload)
            .parse(payload)
            .map_err(|_e| ParseConnectMessageError(()))?;
        Ok(connect_message)
    }
}

/// Connectionless incoming [`Packet`]
pub enum ConnectionlessMessage {
    GetStatus(()),
    GetInfo(()),
    GetChallenge(()),
    Connect(ConnectMessage<InfoString>), // that <KV> generic is annoying here, maybe less so if this were OwnedConnectionlessMessage ?
    IpAuthorize(()),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequencedpacket_new() -> Result<(), Box<dyn std::error::Error>> {
        assert!(
            SequencedPacket::new(PacketSequenceNumber::new(42)?, vec![0; FRAGMENT_SIZE]).is_err()
        );

        assert!(
            SequencedPacket::new(PacketSequenceNumber::new(42)?, vec![0; FRAGMENT_SIZE - 1])
                .is_ok()
        );

        Ok(())
    }

    #[test]
    fn fragmentedpacket_new() -> Result<(), Box<dyn std::error::Error>> {
        assert!(FragmentedPacket::new(
            PacketSequenceNumber::new(42)?,
            FragmentStart::new(42)?,
            vec![0; FRAGMENT_SIZE + 1]
        )
        .is_err());

        assert!(FragmentedPacket::new(
            PacketSequenceNumber::new(42)?,
            FragmentStart::new(42)?,
            vec![0; FRAGMENT_SIZE]
        )
        .is_ok());

        Ok(())
    }

    #[test]
    fn parse_packet_invalidsize() {
        let mut payload = &b"\xFF"[..];

        let packet = parse_packet(&mut payload);
        assert!(matches!(packet, Err(InvalidPacketError::InvalidSize)));
    }

    #[test]
    fn parse_packet_connectionless() -> Result<(), Box<dyn std::error::Error>> {
        let mut payload = &b"\xFF\xFF\xFF\xFF\xDE\xAD\xBE\xEF"[..];

        let packet = parse_packet(&mut payload)?;
        match packet {
            Packet::Connectionless(packet) => {
                assert_eq!(packet.payload(), &b"\xDE\xAD\xBE\xEF"[..]);
            }
            _ => panic!(),
        }

        Ok(())
    }

    #[test]
    fn parse_packet_sequenced() -> Result<(), Box<dyn std::error::Error>> {
        let mut payload = &b"\x00\x00\x00\x00\x9A\x02\xDE\xAD\xBE\xEF"[..];

        let packet = parse_packet(&mut payload)?;
        match packet {
            Packet::Sequenced(packet) => {
                assert_eq!(packet.sequence(), PacketSequenceNumber::new(0)?);
                assert_eq!(packet.qport(), QPort::new(666)?);
                assert_eq!(packet.payload(), &b"\xDE\xAD\xBE\xEF"[..]);
            }
            _ => panic!(),
        }

        Ok(())
    }

    #[test]
    fn parse_packet_fragmented() -> Result<(), Box<dyn std::error::Error>> {
        let mut payload = &b"\x00\x00\x00\x80\x9A\x02\x01\x00\x04\x00\xDE\xAD\xBE\xEF"[..];

        let packet = parse_packet(&mut payload)?;
        match packet {
            Packet::Fragmented(packet) => {
                assert_eq!(packet.sequence(), PacketSequenceNumber::new(0)?);
                assert_eq!(packet.qport(), QPort::new(666)?);
                assert_eq!(
                    packet.fragment_info(),
                    FragmentInfo::new(FragmentStart::new(1)?, FragmentLength::new(4)?)
                );
                assert_eq!(packet.payload(), &b"\xDE\xAD\xBE\xEF"[..]);
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

        let packet = ConnectionlessPacket::new(&encoded_bytes[..])?;
        let connect_message = ConnectMessage::<InfoString>::parse_packet(&packet)?;

        let user_info = connect_message.user_info();

        assert!(user_info.len() == 19);

        Ok(())
    }
}
