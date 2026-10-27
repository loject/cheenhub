//! Общий разбор заголовка v1/v2 без копирования закодированного медиа.

use super::{
    LEGACY_HEADER_LEN, MAGIC, MediaAudioSource, MediaCodec, MediaDatagramError, MediaDatagramKind,
    TRACK_HEADER_LEN, copy_array, validate_track,
};
use uuid::Uuid;

/// Проверенный заголовок медиадатаграммы для owned decoder и серверного relay.
///
/// Первые 64 байта одинаковы у v1 и v2; v2 добавляет UUID, назначение аудио
/// и число каналов. Payload остаётся исходными байтами кодека без вложенной упаковки.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaDatagramHeader {
    /// Вид кадра, определяющий обработчик медиа.
    pub kind: MediaDatagramKind,
    /// Кодек полезной нагрузки.
    pub codec: MediaCodec,
    /// Флаги кодека и фрагментации.
    pub flags: u8,
    /// Последовательность в пределах дорожки.
    pub sequence: u64,
    /// Время захвата в микросекундах.
    pub timestamp_us: u64,
    /// Длительность кадра в микросекундах.
    pub duration_us: u32,
    /// Целевая комната, проверяемая сервером по присутствию отправителя.
    pub room_id: Uuid,
    /// Недоверенный sender ID до замены сервером на аутентифицированный ID.
    pub sender_user_id: Uuid,
    /// UUID дорожки; nil для legacy v1.
    pub track_id: Uuid,
    /// Назначение аудио v2; `None` у видео и legacy v1.
    pub audio_source: Option<MediaAudioSource>,
    /// Число каналов аудио v2; ноль у видео и legacy v1.
    pub audio_channels: u8,
    /// Объявленная длина закодированной полезной нагрузки.
    pub payload_len: usize,
    header_len: usize,
    wire_len: usize,
}

impl MediaDatagramHeader {
    /// Разбирает v1/v2, проверяя размер всего пакета и метаданные дорожки.
    ///
    /// Дополнительные trailing bytes игнорируются, как в legacy decoder.
    ///
    /// # Errors
    /// Возвращает ошибку для усечённого пакета, неизвестного формата или неверной дорожки.
    pub fn decode(bytes: &[u8]) -> Result<Self, MediaDatagramError> {
        if bytes.len() < LEGACY_HEADER_LEN {
            return Err(MediaDatagramError::Truncated);
        }
        if &bytes[..4] != MAGIC {
            return Err(MediaDatagramError::BadMagic);
        }
        let header_len = match bytes[4] {
            1 => LEGACY_HEADER_LEN,
            2 => TRACK_HEADER_LEN,
            version => return Err(MediaDatagramError::UnknownVersion(version)),
        };
        if bytes.len() < header_len {
            return Err(MediaDatagramError::Truncated);
        }
        let kind = MediaDatagramKind::from_u8(bytes[5])?;
        let codec = MediaCodec::from_u8(bytes[6])?;
        let (track_id, audio_source, audio_channels) = if bytes[4] == 2 {
            let track_id = Uuid::from_bytes(copy_array(&bytes[64..80]));
            if track_id.is_nil() {
                return Err(MediaDatagramError::InvalidTrack);
            }
            let source = match bytes[80] {
                0 => None,
                1 => Some(MediaAudioSource::Microphone),
                2 => Some(MediaAudioSource::System),
                _ => return Err(MediaDatagramError::InvalidTrack),
            };
            (track_id, source, bytes[81])
        } else {
            (Uuid::nil(), None, 0)
        };
        validate_track(kind, codec, track_id, audio_source, audio_channels)?;
        let payload_len = u32::from_be_bytes(copy_array(&bytes[60..64])) as usize;
        let wire_len = header_len
            .checked_add(payload_len)
            .ok_or(MediaDatagramError::PayloadTooLarge(payload_len))?;
        if bytes.len() < wire_len {
            return Err(MediaDatagramError::Truncated);
        }
        Ok(Self {
            kind,
            codec,
            flags: bytes[7],
            sequence: u64::from_be_bytes(copy_array(&bytes[8..16])),
            timestamp_us: u64::from_be_bytes(copy_array(&bytes[16..24])),
            duration_us: u32::from_be_bytes(copy_array(&bytes[24..28])),
            room_id: Uuid::from_bytes(copy_array(&bytes[28..44])),
            sender_user_id: Uuid::from_bytes(copy_array(&bytes[44..60])),
            track_id,
            audio_source,
            audio_channels,
            payload_len,
            header_len,
            wire_len,
        })
    }

    /// Возвращает исходный payload без копирования, учитывая версию заголовка.
    ///
    /// # Errors
    /// Возвращает `Truncated`, если передан буфер короче проверенного пакета.
    pub fn payload(self, bytes: &[u8]) -> Result<&[u8], MediaDatagramError> {
        bytes
            .get(self.header_len..self.wire_len)
            .ok_or(MediaDatagramError::Truncated)
    }

    /// Возвращает размер заголовка и payload без trailing bytes.
    pub fn wire_len(self) -> usize {
        self.wire_len
    }
}
