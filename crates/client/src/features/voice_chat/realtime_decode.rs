//! Декодирование входящих realtime-событий и media datagram для голосового чата.

use cheenhub_contracts::media::{
    MEDIA_DATAGRAM_FLAG_KEY_FRAME, MediaCodec, MediaDatagram, MediaDatagramKind,
};
use cheenhub_contracts::realtime::{
    ParticipantNetworkQualityUpdated, RealtimeEnvelope, RealtimeKind, RealtimeModule,
    ServerAudioBitrate, VoiceChatKind, VoiceRoomSnapshot, VoiceVideoStreamEnded,
};

use super::{InboundVideoFrame, InboundVideoStreamEnded, InboundVoiceFrame};

pub(super) fn server_audio_bitrate(envelope: RealtimeEnvelope) -> Option<ServerAudioBitrate> {
    if envelope.module != RealtimeModule::VoiceChat
        || envelope.kind != RealtimeKind::VoiceChat(VoiceChatKind::ServerAudioBitrateUpdated)
    {
        return None;
    }

    serde_json::from_value::<ServerAudioBitrate>(envelope.payload).ok()
}

pub(super) fn participants_changed(envelope: RealtimeEnvelope) -> Option<VoiceRoomSnapshot> {
    if envelope.module != RealtimeModule::VoiceChat
        || envelope.kind != RealtimeKind::VoiceChat(VoiceChatKind::ParticipantsChanged)
    {
        return None;
    }

    serde_json::from_value::<VoiceRoomSnapshot>(envelope.payload).ok()
}

pub(in crate::features::voice_chat) fn participant_network_quality(
    envelope: RealtimeEnvelope,
) -> Option<ParticipantNetworkQualityUpdated> {
    if envelope.module != RealtimeModule::VoiceChat
        || envelope.kind != RealtimeKind::VoiceChat(VoiceChatKind::ParticipantNetworkQualityUpdated)
    {
        return None;
    }

    serde_json::from_value(envelope.payload).ok()
}

pub(super) fn video_stream_ended(envelope: RealtimeEnvelope) -> Option<InboundVideoStreamEnded> {
    if envelope.module != RealtimeModule::VoiceChat
        || envelope.kind != RealtimeKind::VoiceChat(VoiceChatKind::VideoStreamEnded)
    {
        return None;
    }

    let event = serde_json::from_value::<VoiceVideoStreamEnded>(envelope.payload).ok()?;
    Some(InboundVideoStreamEnded {
        server_id: event.server_id,
        room_id: event.room_id,
        sender_user_id: event.user_id,
        source: event.source,
    })
}

pub(super) fn voice_frame(bytes: &[u8]) -> Option<InboundVoiceFrame> {
    let datagram = MediaDatagram::decode(bytes).ok()?;
    if datagram.kind != MediaDatagramKind::VoiceFrame || datagram.codec != MediaCodec::Opus {
        return None;
    }

    Some(InboundVoiceFrame {
        room_id: datagram.room_id.to_string(),
        sender_user_id: datagram.sender_user_id.to_string(),
        sequence: datagram.sequence,
        timestamp_us: datagram.timestamp_us,
        duration_us: datagram.duration_us,
        bytes: datagram.payload,
    })
}

pub(super) fn screen_datagram(bytes: &[u8]) -> Option<MediaDatagram> {
    let datagram = MediaDatagram::decode(bytes).ok()?;
    if datagram.kind != MediaDatagramKind::ScreenFrame || datagram.codec != MediaCodec::Vp9 {
        return None;
    }

    Some(datagram)
}

pub(super) fn camera_datagram(bytes: &[u8]) -> Option<MediaDatagram> {
    let datagram = MediaDatagram::decode(bytes).ok()?;
    if datagram.kind != MediaDatagramKind::CameraFrame || datagram.codec != MediaCodec::Vp9 {
        return None;
    }

    Some(datagram)
}

pub(super) fn video_frame_from_datagram(datagram: MediaDatagram) -> InboundVideoFrame {
    InboundVideoFrame {
        room_id: datagram.room_id.to_string(),
        sender_user_id: datagram.sender_user_id.to_string(),
        sequence: datagram.sequence,
        timestamp_us: datagram.timestamp_us,
        duration_us: datagram.duration_us,
        bytes: datagram.payload,
        key_frame: datagram.flags & MEDIA_DATAGRAM_FLAG_KEY_FRAME != 0,
    }
}

#[cfg(test)]
mod tests {
    use cheenhub_contracts::realtime::{
        ParticipantNetworkQualityUpdated, RealtimeEnvelope, VoiceNetworkTargetKind,
    };

    use super::*;

    #[test]
    fn decodes_participant_network_quality_event() {
        let event = ParticipantNetworkQualityUpdated {
            target_kind: VoiceNetworkTargetKind::Server,
            server_id: "server".to_owned(),
            room_id: "room".to_owned(),
            user_id: "user".to_owned(),
            rtt_ms: 48,
        };
        let envelope = RealtimeEnvelope::new(
            RealtimeModule::VoiceChat,
            RealtimeKind::VoiceChat(VoiceChatKind::ParticipantNetworkQualityUpdated),
            None,
            event.clone(),
        )
        .expect("event serializes");

        assert_eq!(participant_network_quality(envelope), Some(event));
    }
}
