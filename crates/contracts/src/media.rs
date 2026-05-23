//! Shared media datagram contracts.

use uuid::Uuid;

const MAGIC: &[u8; 4] = b"CHUB";
const VERSION: u8 = 1;
const HEADER_LEN: usize = 64;

/// Media datagram flag set when the encoded payload is an independently decodable key frame.
pub const MEDIA_DATAGRAM_FLAG_KEY_FRAME: u8 = 0b0000_0001;

/// Media datagram flag set when the payload carries one fragment of a larger media frame.
pub const MEDIA_DATAGRAM_FLAG_FRAGMENTED: u8 = 0b0000_0010;

/// Media datagram kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaDatagramKind {
    /// Encoded microphone audio frame.
    VoiceFrame = 1,
    /// Encoded screen sharing video frame.
    ScreenFrame = 2,
}

impl MediaDatagramKind {
    fn from_u8(value: u8) -> Result<Self, MediaDatagramError> {
        match value {
            1 => Ok(Self::VoiceFrame),
            2 => Ok(Self::ScreenFrame),
            _ => Err(MediaDatagramError::UnknownKind(value)),
        }
    }
}

/// Encoded media codec carried by a media datagram.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaCodec {
    /// Opus audio.
    Opus = 1,
    /// VP9 video.
    Vp9 = 2,
}

impl MediaCodec {
    fn from_u8(value: u8) -> Result<Self, MediaDatagramError> {
        match value {
            1 => Ok(Self::Opus),
            2 => Ok(Self::Vp9),
            _ => Err(MediaDatagramError::UnknownCodec(value)),
        }
    }
}

/// One decoded media datagram.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaDatagram {
    /// Datagram kind.
    pub kind: MediaDatagramKind,
    /// Encoded payload codec.
    pub codec: MediaCodec,
    /// Codec-specific flags.
    pub flags: u8,
    /// Sender-local packet sequence.
    pub sequence: u64,
    /// Capture or encode timestamp in microseconds.
    pub timestamp_us: u64,
    /// Frame duration in microseconds.
    pub duration_us: u32,
    /// Target room identifier.
    pub room_id: Uuid,
    /// Authenticated sender identifier assigned by the server for relayed frames.
    pub sender_user_id: Uuid,
    /// Raw encoded media payload.
    pub payload: Vec<u8>,
}

impl MediaDatagram {
    /// Encodes this media datagram to its binary wire form.
    pub fn encode(&self) -> Result<Vec<u8>, MediaDatagramError> {
        let payload_len = u32::try_from(self.payload.len())
            .map_err(|_| MediaDatagramError::PayloadTooLarge(self.payload.len()))?;
        let mut bytes = Vec::with_capacity(HEADER_LEN + self.payload.len());
        bytes.extend_from_slice(MAGIC);
        bytes.push(VERSION);
        bytes.push(self.kind as u8);
        bytes.push(self.codec as u8);
        bytes.push(self.flags);
        bytes.extend_from_slice(&self.sequence.to_be_bytes());
        bytes.extend_from_slice(&self.timestamp_us.to_be_bytes());
        bytes.extend_from_slice(&self.duration_us.to_be_bytes());
        bytes.extend_from_slice(self.room_id.as_bytes());
        bytes.extend_from_slice(self.sender_user_id.as_bytes());
        bytes.extend_from_slice(&payload_len.to_be_bytes());
        bytes.extend_from_slice(&self.payload);

        Ok(bytes)
    }

    /// Decodes one binary media datagram.
    pub fn decode(bytes: &[u8]) -> Result<Self, MediaDatagramError> {
        if bytes.len() < HEADER_LEN {
            return Err(MediaDatagramError::Truncated);
        }
        if &bytes[..4] != MAGIC {
            return Err(MediaDatagramError::BadMagic);
        }
        if bytes[4] != VERSION {
            return Err(MediaDatagramError::UnknownVersion(bytes[4]));
        }
        let kind = MediaDatagramKind::from_u8(bytes[5])?;
        let codec = MediaCodec::from_u8(bytes[6])?;
        let flags = bytes[7];
        let sequence = u64::from_be_bytes(copy_array(&bytes[8..16]));
        let timestamp_us = u64::from_be_bytes(copy_array(&bytes[16..24]));
        let duration_us = u32::from_be_bytes(copy_array(&bytes[24..28]));
        let room_id = Uuid::from_bytes(copy_array(&bytes[28..44]));
        let sender_user_id = Uuid::from_bytes(copy_array(&bytes[44..60]));
        let payload_len = u32::from_be_bytes(copy_array(&bytes[60..64])) as usize;
        let expected_len = HEADER_LEN
            .checked_add(payload_len)
            .ok_or(MediaDatagramError::PayloadTooLarge(payload_len))?;
        if bytes.len() < expected_len {
            return Err(MediaDatagramError::Truncated);
        }

        Ok(Self {
            kind,
            codec,
            flags,
            sequence,
            timestamp_us,
            duration_us,
            room_id,
            sender_user_id,
            payload: bytes[HEADER_LEN..expected_len].to_vec(),
        })
    }
}

/// Media datagram encode/decode error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaDatagramError {
    /// Datagram is shorter than the advertised header or payload.
    Truncated,
    /// Datagram does not start with the CheenHub media magic.
    BadMagic,
    /// Datagram version is not supported.
    UnknownVersion(u8),
    /// Datagram kind is not supported.
    UnknownKind(u8),
    /// Datagram codec is not supported.
    UnknownCodec(u8),
    /// Payload length cannot fit the wire format or local allocation.
    PayloadTooLarge(usize),
}

impl std::fmt::Display for MediaDatagramError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Truncated => write!(formatter, "media datagram is truncated"),
            Self::BadMagic => write!(formatter, "media datagram has invalid magic"),
            Self::UnknownVersion(version) => {
                write!(
                    formatter,
                    "media datagram version {version} is not supported"
                )
            }
            Self::UnknownKind(kind) => write!(formatter, "media datagram kind {kind} is unknown"),
            Self::UnknownCodec(codec) => {
                write!(formatter, "media datagram codec {codec} is unknown")
            }
            Self::PayloadTooLarge(size) => {
                write!(
                    formatter,
                    "media datagram payload with {size} bytes is too large"
                )
            }
        }
    }
}

impl std::error::Error for MediaDatagramError {}

fn copy_array<const N: usize>(slice: &[u8]) -> [u8; N] {
    let mut array = [0; N];
    array.copy_from_slice(slice);
    array
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_datagram_round_trips() {
        let datagram = MediaDatagram {
            kind: MediaDatagramKind::VoiceFrame,
            codec: MediaCodec::Opus,
            flags: 0,
            sequence: 42,
            timestamp_us: 123_456,
            duration_us: 20_000,
            room_id: Uuid::new_v4(),
            sender_user_id: Uuid::new_v4(),
            payload: vec![1, 2, 3, 4],
        };

        let encoded = datagram.encode().expect("datagram encodes");
        let decoded = MediaDatagram::decode(&encoded).expect("datagram decodes");

        assert_eq!(decoded, datagram);
    }

    #[test]
    fn screen_media_datagram_round_trips() {
        let datagram = MediaDatagram {
            kind: MediaDatagramKind::ScreenFrame,
            codec: MediaCodec::Vp9,
            flags: MEDIA_DATAGRAM_FLAG_KEY_FRAME,
            sequence: 84,
            timestamp_us: 654_321,
            duration_us: 33_333,
            room_id: Uuid::new_v4(),
            sender_user_id: Uuid::new_v4(),
            payload: vec![9, 8, 7, 6],
        };

        let encoded = datagram.encode().expect("datagram encodes");
        let decoded = MediaDatagram::decode(&encoded).expect("datagram decodes");

        assert_eq!(decoded, datagram);
    }

    #[test]
    fn media_datagram_rejects_truncated_payload() {
        let datagram = MediaDatagram {
            kind: MediaDatagramKind::VoiceFrame,
            codec: MediaCodec::Opus,
            flags: 0,
            sequence: 1,
            timestamp_us: 1,
            duration_us: 20_000,
            room_id: Uuid::new_v4(),
            sender_user_id: Uuid::nil(),
            payload: vec![1, 2, 3],
        };
        let mut encoded = datagram.encode().expect("datagram encodes");
        encoded.pop();

        assert_eq!(
            MediaDatagram::decode(&encoded),
            Err(MediaDatagramError::Truncated)
        );
    }

    #[test]
    fn media_datagram_rejects_unknown_version_kind_and_codec() {
        let datagram = MediaDatagram {
            kind: MediaDatagramKind::VoiceFrame,
            codec: MediaCodec::Opus,
            flags: 0,
            sequence: 1,
            timestamp_us: 1,
            duration_us: 20_000,
            room_id: Uuid::new_v4(),
            sender_user_id: Uuid::nil(),
            payload: vec![],
        };
        let encoded = datagram.encode().expect("datagram encodes");

        let mut unknown_version = encoded.clone();
        unknown_version[4] = 2;
        assert_eq!(
            MediaDatagram::decode(&unknown_version),
            Err(MediaDatagramError::UnknownVersion(2))
        );

        let mut unknown_kind = encoded.clone();
        unknown_kind[5] = 9;
        assert_eq!(
            MediaDatagram::decode(&unknown_kind),
            Err(MediaDatagramError::UnknownKind(9))
        );

        let mut unknown_codec = encoded;
        unknown_codec[6] = 9;
        assert_eq!(
            MediaDatagram::decode(&unknown_codec),
            Err(MediaDatagramError::UnknownCodec(9))
        );
    }
}
