//! Sidebar voice room participant cache helpers.

use cheenhub_contracts::realtime::{VoiceRoomParticipant, VoiceRoomSnapshot};

/// Cached participants for one voice-capable room.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VoiceRoomParticipants {
    server_id: String,
    room_id: String,
    participants: Vec<VoiceRoomParticipant>,
}

/// Returns cached participants for one voice-capable room.
pub(crate) fn participants_for(
    snapshots: &[VoiceRoomParticipants],
    server_id: &str,
    room_id: &str,
) -> Option<Vec<VoiceRoomParticipant>> {
    snapshots.iter().find_map(|snapshot| {
        (snapshot.server_id == server_id && snapshot.room_id == room_id)
            .then(|| snapshot.participants.clone())
    })
}

/// Applies one room participant snapshot to the cache.
pub(crate) fn apply_snapshot(
    snapshots: &mut Vec<VoiceRoomParticipants>,
    snapshot: VoiceRoomSnapshot,
) {
    if snapshot.participants.is_empty() {
        snapshots.retain(|saved| {
            saved.server_id != snapshot.server_id || saved.room_id != snapshot.room_id
        });
    } else if let Some(saved) = snapshots
        .iter_mut()
        .find(|saved| saved.server_id == snapshot.server_id && saved.room_id == snapshot.room_id)
    {
        saved.participants = snapshot.participants;
    } else {
        snapshots.push(VoiceRoomParticipants {
            server_id: snapshot.server_id,
            room_id: snapshot.room_id,
            participants: snapshot.participants,
        });
    }
}

/// Replaces cached active room snapshots for one server.
pub(crate) fn replace_server_snapshots(
    snapshots: &mut Vec<VoiceRoomParticipants>,
    server_id: String,
    next_server_snapshots: Vec<VoiceRoomSnapshot>,
) {
    snapshots.retain(|saved| saved.server_id != server_id);
    snapshots.extend(next_server_snapshots.into_iter().filter_map(|snapshot| {
        (!snapshot.participants.is_empty()).then_some(VoiceRoomParticipants {
            server_id: snapshot.server_id,
            room_id: snapshot.room_id,
            participants: snapshot.participants,
        })
    }));
}
