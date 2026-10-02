//! Общие заготовки видеокадров и присутствия для тестов политики и активности.

use cheenhub_contracts::media::{MediaCodec, MediaDatagram, MediaDatagramKind};
use uuid::Uuid;

use crate::features::voice_chat::infrastructure::{VoicePresence, VoicePresenceTargetKind};

/// Собирает допустимый ключевой кадр заданного вида для указанной комнаты.
pub(crate) fn key_frame_datagram(kind: MediaDatagramKind, room_id: Uuid) -> MediaDatagram {
    sized_key_frame(kind, room_id, 1280, 720)
}

/// Собирает ключевой кадр заданного вида и разрешения.
pub(crate) fn sized_key_frame(
    kind: MediaDatagramKind,
    room_id: Uuid,
    width: u32,
    height: u32,
) -> MediaDatagram {
    MediaDatagram {
        kind,
        codec: MediaCodec::Vp9,
        flags: cheenhub_contracts::media::MEDIA_DATAGRAM_FLAG_KEY_FRAME,
        sequence: 1,
        timestamp_us: 0,
        duration_us: 0,
        room_id,
        sender_user_id: Uuid::nil(),
        payload: vp9_key_frame(width, height),
    }
}

/// Собирает ключевой кадр демонстрации экрана.
pub(crate) fn screen_datagram(room_id: Uuid) -> MediaDatagram {
    key_frame_datagram(MediaDatagramKind::ScreenFrame, room_id)
}

/// Собирает присутствие сессии в комнате для проверки очистки источников.
pub(crate) fn presence(session_id: Uuid, room_id: Uuid) -> VoicePresence {
    VoicePresence {
        realtime_stream_id: Uuid::new_v4(),
        session_id,
        target_kind: VoicePresenceTargetKind::Server,
        server_id: Uuid::new_v4(),
        room_id,
        user_id: Uuid::new_v4(),
        nickname: "video_user".to_owned(),
        avatar_url: None,
        joined_at: chrono::Utc::now(),
    }
}

fn vp9_key_frame(width: u32, height: u32) -> Vec<u8> {
    let mut writer = BitWriter::default();
    writer.write(0b10, 2);
    writer.write(0, 1);
    writer.write(0, 1);
    writer.write(0, 1);
    writer.write(0, 1);
    writer.write(1, 1);
    writer.write(0, 1);
    writer.write(0x49_83_42, 24);
    writer.write(1, 3);
    writer.write(0, 1);
    writer.write(width - 1, 16);
    writer.write(height - 1, 16);
    writer.bytes
}

#[derive(Default)]
struct BitWriter {
    bytes: Vec<u8>,
    bit_offset: usize,
}

impl BitWriter {
    fn write(&mut self, value: u32, count: usize) {
        for bit_index in (0..count).rev() {
            if self.bit_offset.is_multiple_of(8) {
                self.bytes.push(0);
            }
            let bit = ((value >> bit_index) & 1) as u8;
            let byte_index = self.bit_offset / 8;
            let shift = 7 - self.bit_offset % 8;
            self.bytes[byte_index] |= bit << shift;
            self.bit_offset += 1;
        }
    }
}
