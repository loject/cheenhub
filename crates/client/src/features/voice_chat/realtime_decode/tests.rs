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
