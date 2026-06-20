//! Lifecycle локальных видеопотоков голосовой комнаты.

use cheenhub_contracts::realtime::VoiceVideoStreamSource;
use dioxus::prelude::*;

use crate::features::camera::{CameraHandle, CameraStatus};
use crate::features::realtime::RealtimeHandle;
use crate::features::screen_share::{ScreenShareHandle, ScreenShareStatus};

use super::realtime;
use super::state::VoiceRoomTarget;
use super::video_streams::{ParticipantVideoHandle, ParticipantVideoSource};

/// Целевая комната локального video capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LocalVideoTarget {
    server_id: String,
    room_id: String,
    stream_was_live: bool,
}

/// Зависимости для объявления локальных video lifecycle-событий.
#[derive(Clone)]
pub(super) struct LocalVideoRuntime {
    /// Realtime-соединение текущей вкладки.
    pub(super) realtime: RealtimeHandle,
    /// Локальное состояние video tiles участников.
    pub(super) participant_video: ParticipantVideoHandle,
    /// Идентификатор текущего пользователя.
    pub(super) current_user_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocalCaptureStatus {
    Idle,
    Starting,
    Live,
}

/// Синхронизирует жизненный цикл камеры с активной голосовой комнатой.
pub(super) fn reconcile_camera_target(
    camera: CameraHandle,
    runtime: LocalVideoRuntime,
    target_room_signal: &mut Signal<Option<LocalVideoTarget>>,
    active_target: &VoiceRoomTarget,
) {
    let status = match camera.status() {
        CameraStatus::Live => LocalCaptureStatus::Live,
        CameraStatus::Starting => LocalCaptureStatus::Starting,
        CameraStatus::Idle | CameraStatus::PermissionDenied | CameraStatus::Error(_) => {
            LocalCaptureStatus::Idle
        }
    };
    reconcile_local_video_target(
        runtime,
        target_room_signal,
        active_target,
        ParticipantVideoSource::Camera,
        status,
        "stopping camera capture after active voice room changed",
        move || camera.stop(),
    );
}

/// Синхронизирует жизненный цикл демонстрации экрана с активной голосовой комнатой.
pub(super) fn reconcile_screen_share_target(
    screen_share: ScreenShareHandle,
    runtime: LocalVideoRuntime,
    target_room_signal: &mut Signal<Option<LocalVideoTarget>>,
    active_target: &VoiceRoomTarget,
) {
    let status = match screen_share.status() {
        ScreenShareStatus::Live => LocalCaptureStatus::Live,
        ScreenShareStatus::Starting => LocalCaptureStatus::Starting,
        ScreenShareStatus::Idle
        | ScreenShareStatus::PermissionDenied
        | ScreenShareStatus::Error(_) => LocalCaptureStatus::Idle,
    };
    reconcile_local_video_target(
        runtime,
        target_room_signal,
        active_target,
        ParticipantVideoSource::ScreenShare,
        status,
        "stopping screen sharing capture after active voice room changed",
        move || screen_share.stop(),
    );
}

/// Освобождает локальный видеопоток и сообщает об остановке другим участникам.
pub(super) fn release_local_video_target(
    runtime: LocalVideoRuntime,
    target: LocalVideoTarget,
    source: ParticipantVideoSource,
) {
    if !target.stream_was_live {
        return;
    }

    runtime
        .participant_video
        .release_stream(source, &runtime.current_user_id);
    spawn(async move {
        let result = match source {
            ParticipantVideoSource::Camera => {
                realtime::send_camera_stream_stopped(
                    &runtime.realtime,
                    &target.server_id,
                    &target.room_id,
                )
                .await
            }
            ParticipantVideoSource::ScreenShare => {
                realtime::send_screen_stream_stopped(
                    &runtime.realtime,
                    &target.server_id,
                    &target.room_id,
                )
                .await
            }
        };
        if let Err(error) = result {
            warn!(
                %error,
                server_id = %target.server_id,
                room_id = %target.room_id,
                source = source.label(),
                "failed to announce stopped local voice video stream"
            );
        } else {
            debug!(
                server_id = %target.server_id,
                room_id = %target.room_id,
                source = source.label(),
                "announced stopped local voice video stream"
            );
        }
    });
}

/// Конвертирует контрактный источник видео в источник renderer'а участников.
pub(super) fn participant_source_from_contract(
    source: VoiceVideoStreamSource,
) -> ParticipantVideoSource {
    match source {
        VoiceVideoStreamSource::Camera => ParticipantVideoSource::Camera,
        VoiceVideoStreamSource::ScreenShare => ParticipantVideoSource::ScreenShare,
    }
}

fn reconcile_local_video_target(
    runtime: LocalVideoRuntime,
    target_room_signal: &mut Signal<Option<LocalVideoTarget>>,
    active_target: &VoiceRoomTarget,
    source: ParticipantVideoSource,
    status: LocalCaptureStatus,
    room_changed_log: &'static str,
    stop_capture: impl FnOnce(),
) {
    match status {
        LocalCaptureStatus::Live => match target_room_signal() {
            Some(target) if target.matches(active_target) && target.stream_was_live => {}
            Some(target) if target.matches(active_target) => {
                target_room_signal.set(Some(target.mark_live()));
            }
            Some(previous_target) => {
                release_local_video_target(runtime, previous_target, source);
                info!("{room_changed_log}");
                target_room_signal.set(None);
                stop_capture();
            }
            None => target_room_signal.set(Some(LocalVideoTarget::from_voice_target(
                active_target,
                true,
            ))),
        },
        LocalCaptureStatus::Starting => match target_room_signal() {
            Some(target) if target.matches(active_target) => {}
            Some(previous_target) => {
                release_local_video_target(runtime, previous_target, source);
                info!("{room_changed_log}");
                target_room_signal.set(None);
                stop_capture();
            }
            None => target_room_signal.set(Some(LocalVideoTarget::from_voice_target(
                active_target,
                false,
            ))),
        },
        LocalCaptureStatus::Idle => {
            if let Some(target) = target_room_signal() {
                release_local_video_target(runtime, target, source);
                target_room_signal.set(None);
            }
        }
    }
}

impl LocalVideoTarget {
    fn from_voice_target(target: &VoiceRoomTarget, stream_was_live: bool) -> Self {
        Self {
            server_id: target.server_id.clone(),
            room_id: target.room_id.clone(),
            stream_was_live,
        }
    }

    fn matches(&self, target: &VoiceRoomTarget) -> bool {
        self.server_id == target.server_id && self.room_id == target.room_id
    }

    fn mark_live(self) -> Self {
        Self {
            stream_was_live: true,
            ..self
        }
    }
}
