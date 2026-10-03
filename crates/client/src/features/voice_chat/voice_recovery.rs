//! Восстановление голоса после потери realtime-соединения.
//!
//! Модуль не входит в `provider.rs`, потому что разрушение и восстановление
//! голосового присутствия — самостоятельное поведение: при обрыве транспорта
//! оно сбрасывает локальное состояние медиа, а после восстановления соединения
//! возвращает пользователя в ту же комнату.

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::*;
use futures_util::StreamExt;

use crate::features::audio_playback::AudioPlaybackHandle;
use crate::features::realtime::{RealtimeConnectionStatus, RealtimeHandle};

use super::notification_sounds::ConnectionNotificationSoundState;
use super::state::{
    VoiceConnectionHandle, VoiceConnectionState, VoiceRoomTarget, VoiceRoomTargetKind,
};
use super::video_streams::ParticipantVideoHandle;

/// Поддерживает голосовое присутствие при обрывах и восстановлении realtime.
///
/// При отключении запоминается комната, в которой пользователь был до обрыва,
/// и очищается локальное состояние медиа. После успешного переподключения
/// выполняется повторный вход в запомненную комнату, поэтому плановый
/// перезапуск backend не оставляет пользователя без голоса.
///
/// Личные звонки не восстанавливаются здесь: ими владеет модуль личных звонков,
/// который после переподключения запрашивает снимок активных звонков и сам
/// возвращает пользователя в комнату.
pub(super) fn use_voice_recovery(
    handle: VoiceConnectionHandle,
    realtime: RealtimeHandle,
    mut state: Signal<VoiceConnectionState>,
    playback: AudioPlaybackHandle,
    participant_video: ParticipantVideoHandle,
    connection_sounds: Rc<RefCell<ConnectionNotificationSoundState>>,
) {
    use_hook(move || {
        spawn(async move {
            let mut statuses = realtime.subscribe_connection_status();
            while let Some(status) = statuses.next().await {
                let connected = matches!(status, RealtimeConnectionStatus::Connected(_));
                if connected {
                    rejoin_remembered_room(&handle);
                } else if matches!(status, RealtimeConnectionStatus::Disconnected) {
                    let mut recovery_handle = handle.clone();
                    clear_voice_after_disconnect(
                        &mut recovery_handle,
                        &mut state,
                        &playback,
                        &participant_video,
                    );
                }

                let voice_chat_active = handle.state().active_target().is_some();
                connection_sounds
                    .borrow_mut()
                    .record(connected, voice_chat_active, &playback);
            }
        })
    });
}

/// Возвращает пользователя в комнату, в которой он был до обрыва транспорта.
///
/// Повторный вход выполняется только для серверных голосовых комнат и только
/// если нет другой активной цели. Запомненная цель забирается один раз,
/// чтобы восстановление не отменяло новый выбор пользователя.
fn rejoin_remembered_room(handle: &VoiceConnectionHandle) {
    let remembered = handle.take_remembered_room();
    let active = handle.state().active_target().is_some();
    if !should_rejoin(remembered.as_ref(), active) {
        return;
    }

    let target = remembered.expect("checked by should_rejoin");
    info!(
        server_id = %target.server_id,
        room_id = %target.room_id,
        "rejoining voice room after realtime reconnect"
    );
    handle.join(target);
}

/// Определяет, нужно ли повторно входить в комнату после переподключения.
///
/// Повторный вход выполняется только для серверных голосовых комнат: личные
/// звонки восстанавливает модуль звонков по снимку активных звонков. Если
/// присутствие уже восстановлено, второй join-запрос в комнату не отправляется.
fn should_rejoin(remembered: Option<&VoiceRoomTarget>, already_active: bool) -> bool {
    let Some(target) = remembered else {
        return false;
    };
    target.kind == VoiceRoomTargetKind::Server && !already_active
}

/// Очищает локальное голосовое состояние после разрыва транспорта.
///
/// Активная комната запоминается до сброса состояния, чтобы её можно было
/// восстановить после переподключения.
fn clear_voice_after_disconnect(
    handle: &mut VoiceConnectionHandle,
    state: &mut Signal<VoiceConnectionState>,
    playback: &AudioPlaybackHandle,
    participant_video: &ParticipantVideoHandle,
) {
    remember_active_room(handle);

    state.set(VoiceConnectionState::Disconnected);
    handle.clear_speaking_users();
    participant_video.clear();
    playback.stop_all();
}

/// Запоминает присутствие для следующего восстановления транспорта.
///
/// Только незавершённый вход или установленное присутствие восстанавливаются;
/// ручной выход и ошибка входа не становятся новой целью.
fn remember_active_room(handle: &mut VoiceConnectionHandle) {
    let room = match handle.state() {
        VoiceConnectionState::Connecting { target }
        | VoiceConnectionState::Connected { target, .. } => target,
        _ => return,
    };
    info!(
        target_kind = ?room.kind,
        server_id = %room.server_id,
        room_id = %room.room_id,
        "remembering voice room before realtime disconnect"
    );
    handle.remember_room(room);
}

#[cfg(test)]
mod tests;
