//! Общие контракты медиадатаграмм.

use uuid::Uuid;

mod header;

pub use header::MediaDatagramHeader;

const MAGIC: &[u8; 4] = b"CHUB";
const LEGACY_HEADER_LEN: usize = 64;
const TRACK_HEADER_LEN: usize = 82;

/// Назначение аудиодорожки; не заменяет серверную проверку права публикации.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MediaAudioSource {
    /// Микрофон с независимым mute и обработкой речи.
    Microphone = 1,
    /// Системный звук, связанный с демонстрацией целого экрана.
    System = 2,
}

/// Флаг медиадатаграммы, когда закодированная полезная нагрузка является независимо декодируемым ключевым кадром.
pub const MEDIA_DATAGRAM_FLAG_KEY_FRAME: u8 = 0b0000_0001;

/// Флаг медиадатаграммы, когда полезная нагрузка несет один фрагмент более крупного медиакадра.
pub const MEDIA_DATAGRAM_FLAG_FRAGMENTED: u8 = 0b0000_0010;

/// Целевой битрейт Opus-кодировщика голосовых сообщений в битах в секунду.
pub const VOICE_AUDIO_BITRATE_BPS: u32 = 32_000;

/// Минимальный допустимый целевой битрейт Opus-аудио сервера в битах в секунду.
pub const VOICE_AUDIO_BITRATE_MIN_BPS: u32 = 16_000;

/// Максимальный допустимый целевой битрейт Opus-аудио сервера в битах в секунду.
pub const VOICE_AUDIO_BITRATE_MAX_BPS: u32 = 96_000;

/// Вид медиадатаграммы.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MediaDatagramKind {
    /// Закодированный кадр аудиодорожки микрофона или системного звука.
    VoiceFrame = 1,
    /// Закодированный видеокадр демонстрации экрана.
    ScreenFrame = 2,
    /// Закодированный видеокадр камеры.
    CameraFrame = 3,
}

impl MediaDatagramKind {
    fn from_u8(value: u8) -> Result<Self, MediaDatagramError> {
        match value {
            1 => Ok(Self::VoiceFrame),
            2 => Ok(Self::ScreenFrame),
            3 => Ok(Self::CameraFrame),
            _ => Err(MediaDatagramError::UnknownKind(value)),
        }
    }
}

/// Закодированный медиакодек, передаваемый медиадатаграммой.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaCodec {
    /// Аудио Opus.
    Opus = 1,
    /// Видео VP9.
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

/// Одна декодированная медиадатаграмма.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaDatagram {
    /// Вид датаграммы.
    pub kind: MediaDatagramKind,
    /// Кодек закодированной полезной нагрузки.
    pub codec: MediaCodec,
    /// Флаги, специфичные для кодека.
    pub flags: u8,
    /// Последовательность пакетов в пределах одной дорожки отправителя.
    pub sequence: u64,
    /// Временная метка захвата или кодирования в микросекундах.
    pub timestamp_us: u64,
    /// Длительность кадра в микросекундах.
    pub duration_us: u32,
    /// Идентификатор целевой комнаты.
    pub room_id: Uuid,
    /// Идентификатор аутентифицированного отправителя, назначенный сервером для ретранслируемых кадров.
    pub sender_user_id: Uuid,
    /// UUID захвата, общий для пакетов одной дорожки; nil обозначает legacy-пакет v1.
    pub track_id: Uuid,
    /// Назначение аудио v2; `None` для видео и legacy-пакетов v1.
    pub audio_source: Option<MediaAudioSource>,
    /// Число каналов аудио v2 (1 или 2); ноль для видео и legacy-пакетов v1.
    pub audio_channels: u8,
    /// Сырая закодированная медиа-полезная нагрузка.
    pub payload: Vec<u8>,
}

impl MediaDatagram {
    /// Кодирует v2 для именованной дорожки и v1 для legacy-пакета с nil UUID.
    ///
    /// # Errors
    /// Возвращает ошибку при некорректных метаданных дорожки или слишком большом payload.
    pub fn encode(&self) -> Result<Vec<u8>, MediaDatagramError> {
        validate_track(
            self.kind,
            self.codec,
            self.track_id,
            self.audio_source,
            self.audio_channels,
        )?;
        let tracked = !self.track_id.is_nil();
        let header_len = if tracked {
            TRACK_HEADER_LEN
        } else {
            LEGACY_HEADER_LEN
        };
        let payload_len = u32::try_from(self.payload.len())
            .map_err(|_| MediaDatagramError::PayloadTooLarge(self.payload.len()))?;
        let mut bytes = Vec::with_capacity(header_len + self.payload.len());
        bytes.extend_from_slice(MAGIC);
        bytes.push(if tracked { 2 } else { 1 });
        bytes.push(self.kind as u8);
        bytes.push(self.codec as u8);
        bytes.push(self.flags);
        bytes.extend_from_slice(&self.sequence.to_be_bytes());
        bytes.extend_from_slice(&self.timestamp_us.to_be_bytes());
        bytes.extend_from_slice(&self.duration_us.to_be_bytes());
        bytes.extend_from_slice(self.room_id.as_bytes());
        bytes.extend_from_slice(self.sender_user_id.as_bytes());
        bytes.extend_from_slice(&payload_len.to_be_bytes());
        if tracked {
            bytes.extend_from_slice(self.track_id.as_bytes());
            bytes.push(self.audio_source.map_or(0, |source| source as u8));
            bytes.push(self.audio_channels);
        }
        bytes.extend_from_slice(&self.payload);

        Ok(bytes)
    }

    /// Декодирует v1/v2; legacy-аудио получает nil UUID, `None` и ноль каналов.
    ///
    /// # Errors
    /// Возвращает ошибку заголовка, метаданных или усечённой полезной нагрузки.
    pub fn decode(bytes: &[u8]) -> Result<Self, MediaDatagramError> {
        let header = MediaDatagramHeader::decode(bytes)?;
        Ok(Self {
            kind: header.kind,
            codec: header.codec,
            flags: header.flags,
            sequence: header.sequence,
            timestamp_us: header.timestamp_us,
            duration_us: header.duration_us,
            room_id: header.room_id,
            sender_user_id: header.sender_user_id,
            track_id: header.track_id,
            audio_source: header.audio_source,
            audio_channels: header.audio_channels,
            payload: header.payload(bytes)?.to_vec(),
        })
    }
}

/// Ошибка кодирования/декодирования медиадатаграммы.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaDatagramError {
    /// Датаграмма короче заявленного заголовка или полезной нагрузки.
    Truncated,
    /// Датаграмма не начинается с media magic CheenHub.
    BadMagic,
    /// Версия датаграммы не поддерживается.
    UnknownVersion(u8),
    /// Вид датаграммы не поддерживается.
    UnknownKind(u8),
    /// Кодек датаграммы не поддерживается.
    UnknownCodec(u8),
    /// UUID, назначение или число каналов не соответствует виду медиадорожки.
    InvalidTrack,
    /// Длина полезной нагрузки не помещается в wire-формат или локальное выделение.
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
            Self::InvalidTrack => write!(formatter, "media datagram track metadata is invalid"),
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

fn validate_track(
    kind: MediaDatagramKind,
    codec: MediaCodec,
    track_id: Uuid,
    source: Option<MediaAudioSource>,
    channels: u8,
) -> Result<(), MediaDatagramError> {
    let valid = if track_id.is_nil() {
        source.is_none() && channels == 0
    } else if kind == MediaDatagramKind::VoiceFrame {
        codec == MediaCodec::Opus && source.is_some() && matches!(channels, 1 | 2)
    } else {
        source.is_none() && channels == 0
    };
    if valid {
        Ok(())
    } else {
        Err(MediaDatagramError::InvalidTrack)
    }
}

fn copy_array<const N: usize>(slice: &[u8]) -> [u8; N] {
    let mut array = [0; N];
    array.copy_from_slice(slice);
    array
}

#[cfg(test)]
mod tests;
