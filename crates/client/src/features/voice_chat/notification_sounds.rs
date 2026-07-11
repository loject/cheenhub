//! Сопоставление событий голосовой комнаты с короткими звуками.

use std::collections::HashSet;

use cheenhub_contracts::realtime::VoiceRoomParticipant;
use dioxus::prelude::debug;

use crate::features::audio_playback::{AudioPlaybackHandle, NotificationSound};

use super::state::VoiceRoomTarget;

/// Хранит последний озвученный состав активной голосовой комнаты.
#[derive(Default)]
pub(super) struct VoiceNotificationSoundState {
    room_key: Option<String>,
    participant_ids: HashSet<String>,
    current_user_connected: bool,
}

impl VoiceNotificationSoundState {
    /// Обновляет состояние подключенной комнаты и проигрывает звуки входа участников.
    pub(super) fn record_connected(
        &mut self,
        target: &VoiceRoomTarget,
        participants: &[VoiceRoomParticipant],
        current_user_id: &str,
        playback: &AudioPlaybackHandle,
    ) {
        let room_key = room_key(target);
        let participant_ids = participant_ids(participants);
        let same_room = self.room_key.as_deref() == Some(room_key.as_str());

        if !same_room {
            if !self.current_user_connected {
                playback.play_notification_sound(NotificationSound::CurrentUserJoined);
            }
            self.room_key = Some(room_key);
            self.participant_ids = participant_ids;
            self.current_user_connected = true;
            return;
        }

        for user_id in participant_ids.difference(&self.participant_ids) {
            if user_id != current_user_id {
                playback.play_notification_sound(NotificationSound::OtherUserJoined);
            }
        }
        for user_id in self.participant_ids.difference(&participant_ids) {
            if user_id != current_user_id {
                playback.play_notification_sound(NotificationSound::OtherUserLeft);
            }
        }

        self.participant_ids = participant_ids;
        self.current_user_connected = true;
    }

    /// Возвращает, был ли текущий пользователь отмечен подключенным к голосовой комнате.
    pub(super) fn is_current_user_connected(&self) -> bool {
        self.current_user_connected
    }

    /// Отмечает выход из активной комнаты и возвращает звук выхода после cleanup'а.
    pub(super) fn record_inactive(&mut self, playback: &AudioPlaybackHandle) {
        if self.current_user_connected {
            playback.play_notification_sound(NotificationSound::CurrentUserLeft);
            debug!("recorded current user voice room leave notification");
        }
        self.room_key = None;
        self.participant_ids.clear();
        self.current_user_connected = false;
    }
}

/// Хранит последний озвученный булевый статус.
#[derive(Default)]
pub(super) struct ToggleNotificationSoundState {
    initialized: bool,
    active: bool,
}

impl ToggleNotificationSoundState {
    /// Проигрывает звук только при реальном изменении статуса.
    pub(super) fn record(
        &mut self,
        active: bool,
        enabled_sound: NotificationSound,
        disabled_sound: NotificationSound,
        playback: &AudioPlaybackHandle,
    ) {
        if !self.initialized {
            self.initialized = true;
            self.active = active;
            return;
        }
        if self.active == active {
            return;
        }

        self.active = active;
        playback.play_notification_sound(if active {
            enabled_sound
        } else {
            disabled_sound
        });
    }
}

/// Хранит состояние звуков потери и восстановления realtime-соединения.
#[derive(Default)]
pub(super) struct ConnectionNotificationSoundState {
    has_connected: bool,
    lost_after_connect: bool,
}

impl ConnectionNotificationSoundState {
    /// Проигрывает lost/restored только после первого успешного соединения.
    pub(super) fn record(
        &mut self,
        connected: bool,
        voice_chat_active: bool,
        playback: &AudioPlaybackHandle,
    ) {
        if connected {
            if self.lost_after_connect {
                playback.stop_connection_signal_loop();
                playback.play_notification_sound(NotificationSound::ConnectionRestored);
            }
            self.has_connected = true;
            self.lost_after_connect = false;
            return;
        }

        if self.has_connected && !self.lost_after_connect {
            playback.play_notification_sound(NotificationSound::ConnectionLost);
            if voice_chat_active {
                playback.start_connection_signal_loop();
                debug!("started connection signal loop for active voice chat");
            }
            self.lost_after_connect = true;
        }
    }
}

fn room_key(target: &VoiceRoomTarget) -> String {
    format!("{}:{}", target.server_id, target.room_id)
}

fn participant_ids(participants: &[VoiceRoomParticipant]) -> HashSet<String> {
    participants
        .iter()
        .map(|participant| participant.user_id.clone())
        .collect()
}
