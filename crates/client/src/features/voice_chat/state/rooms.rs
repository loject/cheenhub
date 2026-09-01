//! Доступ к кэшу снимков присутствия комнат и их загрузка по realtime.

use dioxus::prelude::*;
use futures_util::StreamExt;

use crate::features::realtime::{RealtimeConnectionStatus, RealtimeHandle, RealtimeTransportKind};

use super::VoiceConnectionHandle;
use crate::features::voice_chat::{realtime, room_presence};

impl VoiceConnectionHandle {
    /// Returns the latest known participant list for one voice-capable room.
    pub(crate) fn room_participants(
        &self,
        server_id: &str,
        room_id: &str,
    ) -> Option<Vec<cheenhub_contracts::realtime::VoiceRoomParticipant>> {
        room_presence::participants_for(&(self.room_snapshots)(), server_id, room_id)
    }

    /// Loads active voice room snapshots for a server over realtime.
    pub(crate) fn load_server_voice_rooms(&self, server_id: String) {
        let realtime = self.realtime.clone();
        let handle = self.clone();
        spawn(async move {
            if !matches!(
                realtime.connection_status(),
                RealtimeConnectionStatus::Connected(_)
            ) {
                info!(
                    server_id = %server_id,
                    "waiting for realtime before loading server voice room sidebar participants"
                );
            }
            let Some(transport) = wait_for_realtime_connection(&realtime).await else {
                warn!(
                    server_id = %server_id,
                    "realtime status subscription closed before server voice room sidebar participants could load"
                );
                return;
            };
            debug!(
                ?transport,
                server_id = %server_id,
                "realtime is ready; loading server voice room sidebar participants"
            );
            match realtime::list_server_voice_rooms(&realtime, server_id.clone()).await {
                Ok(snapshot) => {
                    info!(
                        server_id = %snapshot.server_id,
                        active_voice_rooms = snapshot.rooms.len(),
                        "loaded server voice room sidebar participants"
                    );
                    handle.replace_server_room_snapshots(snapshot.server_id, snapshot.rooms);
                }
                Err(error) => {
                    warn!(
                        %error,
                        server_id = %server_id,
                        "failed to load server voice room sidebar participants"
                    );
                }
            }
        });
    }

    /// Loads active direct-message voice room snapshots.
    pub(crate) fn load_direct_message_voice_rooms(&self) {
        let realtime = self.realtime.clone();
        let handle = self.clone();
        spawn(async move {
            match realtime::list_direct_message_voice_rooms(&realtime).await {
                Ok(snapshot) => {
                    info!(
                        active_voice_rooms = snapshot.rooms.len(),
                        "loaded direct message voice room participants"
                    );
                    for room in snapshot.rooms {
                        handle.apply_room_snapshot(room);
                    }
                }
                Err(error) => {
                    warn!(%error, "failed to load direct message voice room participants");
                }
            }
        });
    }

    pub(super) fn apply_room_snapshot(
        &self,
        snapshot: cheenhub_contracts::realtime::VoiceRoomSnapshot,
    ) {
        let mut next_snapshots = (self.room_snapshots)();
        room_presence::apply_snapshot(&mut next_snapshots, snapshot);
        let mut room_snapshots = self.room_snapshots;
        room_snapshots.set(next_snapshots);
    }

    pub(super) fn replace_server_room_snapshots(
        &self,
        server_id: String,
        snapshots: Vec<cheenhub_contracts::realtime::VoiceRoomSnapshot>,
    ) {
        let mut next_snapshots = (self.room_snapshots)();
        room_presence::replace_server_snapshots(&mut next_snapshots, server_id, snapshots);
        let mut room_snapshots = self.room_snapshots;
        room_snapshots.set(next_snapshots);
    }
}

async fn wait_for_realtime_connection(realtime: &RealtimeHandle) -> Option<RealtimeTransportKind> {
    let mut statuses = realtime.subscribe_connection_status();
    while let Some(status) = statuses.next().await {
        if let RealtimeConnectionStatus::Connected(transport) = status {
            return Some(transport);
        }
    }

    None
}
