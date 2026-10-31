//! Общее состояние и renderer видеопотоков участников.

mod backend;
mod canvas;
mod delivery;
mod native;
mod web;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use dioxus::core::{Runtime, ScopeId};
use dioxus::prelude::*;
use futures_channel::mpsc;
use uuid::Uuid;

use crate::features::camera::EncodedCameraFrame;
use crate::features::runtime::sleep_ms;
use crate::features::screen_share::EncodedScreenShareFrame;

use self::backend::{ParticipantVideoBackend, ParticipantVideoRenderer};
pub(crate) use self::canvas::ParticipantVideoCanvas;
use super::realtime::InboundVideoFrame;

const PARTICIPANT_VIDEO_RELEASE_TIMEOUT_MS: u32 = 1_500;

type ParticipantVideoSubscribers =
    Rc<RefCell<HashMap<ParticipantVideoKey, delivery::ParticipantVideoDelivery>>>;
type ParticipantVideoGenerations = Rc<RefCell<HashMap<ParticipantVideoKey, u64>>>;
type ParticipantVideoBlockedStreams =
    Rc<RefCell<HashMap<ParticipantVideoKey, ParticipantVideoBlockReason>>>;

/// Причина блокировки кадров: остановленный источник можно перезапустить,
/// завершённую голосовую сессию нельзя оживить поздним ключевым кадром.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::features::voice_chat) enum ParticipantVideoBlockReason {
    AwaitingKeyFrame,
    DepartedPresence,
}

/// Тип видеопотока внутри плитки участника.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ParticipantVideoSource {
    /// Поток камеры участника.
    Camera,
    /// Поток демонстрации экрана участника.
    ScreenShare,
}

impl ParticipantVideoSource {
    /// Возвращает человекочитаемую метку источника для логов и ошибок.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Camera => "camera",
            Self::ScreenShare => "screen_share",
        }
    }

    fn id_prefix(self) -> &'static str {
        match self {
            Self::Camera => "camera-video",
            Self::ScreenShare => "screen-video",
        }
    }
}

/// Один закодированный кадр видеопотока участника.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParticipantVideoFrame {
    /// Идентификатор целевой комнаты.
    pub(crate) room_id: String,
    /// Идентификатор аутентифицированного отправителя.
    pub(crate) sender_user_id: String,
    /// UUID видеодорожки, равный голосовой сессии отправителя; nil для локального preview.
    pub(crate) track_id: Uuid,
    /// Локальная для отправителя последовательность пакетов.
    pub(crate) sequence: u64,
    /// Временная метка захвата или кодирования в микросекундах.
    pub(crate) timestamp_us: u64,
    /// Длительность кадра в микросекундах.
    pub(crate) duration_us: u32,
    /// Сырые байты закодированного VP9 кадра.
    pub(crate) bytes: Vec<u8>,
    /// Может ли этот кадр открыть поток декодера.
    pub(crate) key_frame: bool,
}

impl ParticipantVideoFrame {
    /// Создает локальный кадр камеры текущего пользователя.
    pub(crate) fn from_local_camera(
        room_id: String,
        sender_user_id: String,
        frame: EncodedCameraFrame,
    ) -> Self {
        Self {
            room_id,
            sender_user_id,
            track_id: Uuid::nil(),
            sequence: frame.sequence,
            timestamp_us: frame.timestamp_us,
            duration_us: frame.duration_us,
            bytes: frame.bytes,
            key_frame: frame.key_frame,
        }
    }

    /// Создает локальный кадр демонстрации экрана текущего пользователя.
    pub(crate) fn from_local_screen_share(
        room_id: String,
        sender_user_id: String,
        frame: EncodedScreenShareFrame,
    ) -> Self {
        Self {
            room_id,
            sender_user_id,
            track_id: Uuid::nil(),
            sequence: frame.sequence,
            timestamp_us: frame.timestamp_us,
            duration_us: frame.duration_us,
            bytes: frame.bytes,
            key_frame: frame.key_frame,
        }
    }
}

impl From<InboundVideoFrame> for ParticipantVideoFrame {
    fn from(frame: InboundVideoFrame) -> Self {
        Self {
            room_id: frame.room_id,
            sender_user_id: frame.sender_user_id,
            track_id: frame.track_id,
            sequence: frame.sequence,
            timestamp_us: frame.timestamp_us,
            duration_us: frame.duration_us,
            bytes: frame.bytes,
            key_frame: frame.key_frame,
        }
    }
}

/// Метка активности одного видеоисточника участника.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParticipantVideoActivity {
    source: ParticipantVideoSource,
    user_id: String,
    track_id: Uuid,
}

/// Состояние видео участников внутри функции голосового чата.
#[derive(Clone)]
pub(crate) struct ParticipantVideoHandle {
    owner_scope: ScopeId,
    live_streams: Signal<Vec<ParticipantVideoActivity>>,
    subscribers: ParticipantVideoSubscribers,
    generations: ParticipantVideoGenerations,
    blocked_streams: ParticipantVideoBlockedStreams,
    backend: Rc<dyn ParticipantVideoBackend>,
}

impl ParticipantVideoHandle {
    /// Создает состояние видео участников.
    pub(crate) fn new(
        live_streams: Signal<Vec<ParticipantVideoActivity>>,
        subscribers: ParticipantVideoSubscribers,
        generations: ParticipantVideoGenerations,
        blocked_streams: ParticipantVideoBlockedStreams,
        owner_scope: ScopeId,
    ) -> Self {
        Self {
            owner_scope,
            live_streams,
            subscribers,
            generations,
            blocked_streams,
            backend: native::default_backend(),
        }
    }

    /// Возвращает активные видеодорожки с идентификатором пользователя и сессии.
    pub(crate) fn live_tracks(&self, source: ParticipantVideoSource) -> Vec<(String, Uuid)> {
        (self.live_streams)()
            .into_iter()
            .filter(|activity| activity.source == source)
            .map(|activity| (activity.user_id, activity.track_id))
            .collect()
    }

    /// Публикует кадр видео в подходящую плитку участника.
    pub(crate) fn publish_frame(
        &self,
        source: ParticipantVideoSource,
        frame: ParticipantVideoFrame,
    ) {
        Runtime::current().in_scope(self.owner_scope, || {
            let key =
                ParticipantVideoKey::new(source, frame.sender_user_id.clone(), frame.track_id);
            if self.should_drop_blocked_frame(&key, frame.key_frame) {
                debug!(
                    user_id = %key.user_id,
                    source = key.source.label(),
                    sequence = frame.sequence,
                    "dropped participant video frame until next key frame"
                );
                return;
            }
            self.subscribers
                .borrow_mut()
                .entry(key.clone())
                .or_default()
                .publish(frame);
            self.mark_live(key);
        });
    }

    /// Останавливает одну сессию; старое событие без UUID очищает источник пользователя.
    pub(crate) fn release_stream_for_event(
        &self,
        source: ParticipantVideoSource,
        user_id: &str,
        session_id: Uuid,
    ) {
        if !session_id.is_nil() {
            self.release_stream(source, user_id, session_id);
            return;
        }
        for (stream_user_id, track_id) in self.live_tracks(source) {
            if stream_user_id == user_id {
                self.release_stream(source, user_id, track_id);
            }
        }
    }

    /// Немедленно освобождает индикатор активности одного видеопотока.
    pub(crate) fn release_stream(
        &self,
        source: ParticipantVideoSource,
        user_id: &str,
        track_id: Uuid,
    ) {
        let key = ParticipantVideoKey::new(source, user_id.to_owned(), track_id);
        self.generations.borrow_mut().remove(&key);
        self.blocked_streams
            .borrow_mut()
            .insert(key.clone(), ParticipantVideoBlockReason::AwaitingKeyFrame);
        self.subscribers.borrow_mut().remove(&key);
        let mut next_streams = (self.live_streams)();
        let previous_len = next_streams.len();
        next_streams.retain(|activity| {
            activity.source != key.source
                || activity.user_id != key.user_id
                || activity.track_id != key.track_id
        });
        if next_streams.len() == previous_len {
            return;
        }

        let mut live_streams = self.live_streams;
        live_streams.set(next_streams);
        debug!(
            user_id = %key.user_id,
            source = key.source.label(),
            "released participant video stream live marker"
        );
    }

    /// Запрещает поздним кадрам повторно открыть видео покинувшей сессии.
    fn retire_stream_for_event(
        &self,
        source: ParticipantVideoSource,
        user_id: &str,
        session_id: Uuid,
    ) {
        let tracks = if session_id.is_nil() {
            self.live_tracks(source)
                .into_iter()
                .filter_map(|(owner, id)| (owner == user_id).then_some(id))
                .collect()
        } else {
            vec![session_id]
        };
        for track_id in tracks {
            self.release_stream(source, user_id, track_id);
            self.blocked_streams.borrow_mut().insert(
                ParticipantVideoKey::new(source, user_id.to_owned(), track_id),
                ParticipantVideoBlockReason::DepartedPresence,
            );
        }
    }

    /// Сбрасывает только видео завершившихся голосовых сессий участников.
    ///
    /// Неподвижная демонстрация остаётся видимой, пока в снимке комнаты
    /// сохраняются та же сессия и время входа, даже если давно не было кадров.
    pub(crate) fn reconcile_participants(
        &self,
        previous: &[cheenhub_contracts::realtime::VoiceRoomParticipant],
        current: &[cheenhub_contracts::realtime::VoiceRoomParticipant],
    ) {
        for participant in previous {
            let still_same_session = current.iter().any(|next| {
                next.user_id == participant.user_id
                    && next.session_id == participant.session_id
                    && next.joined_at == participant.joined_at
            });
            if still_same_session {
                continue;
            }
            let track_id = match Uuid::parse_str(&participant.session_id) {
                Ok(id) => id,
                Err(_) if participant.session_id.is_empty() => Uuid::nil(),
                Err(_) => {
                    warn!(
                        user_id = %participant.user_id,
                        session_id = %participant.session_id,
                        "cannot release participant video for invalid voice session id"
                    );
                    continue;
                }
            };

            self.retire_stream_for_event(
                ParticipantVideoSource::Camera,
                &participant.user_id,
                track_id,
            );
            self.retire_stream_for_event(
                ParticipantVideoSource::ScreenShare,
                &participant.user_id,
                track_id,
            );
            debug!(
                user_id = %participant.user_id,
                %track_id,
                joined_at = %participant.joined_at,
                "cleared participant video sources after voice presence changed"
            );
        }
        for participant in current {
            let same_presence = previous.iter().any(|old| {
                old.user_id == participant.user_id
                    && old.session_id == participant.session_id
                    && old.joined_at == participant.joined_at
            });
            if same_presence {
                continue;
            }
            let Ok(session_id) = Uuid::parse_str(&participant.session_id) else {
                continue;
            };
            self.blocked_streams.borrow_mut().retain(|key, reason| {
                *reason != ParticipantVideoBlockReason::DepartedPresence
                    || key.user_id != participant.user_id
                    || key.track_id != session_id
            });
        }
    }

    /// Подписывает плитку участника на один видеопоток.
    pub(crate) fn subscribe_stream(
        &self,
        source: ParticipantVideoSource,
        user_id: String,
        track_id: Uuid,
    ) -> mpsc::UnboundedReceiver<ParticipantVideoFrame> {
        self.subscribers
            .borrow_mut()
            .entry(ParticipantVideoKey::new(source, user_id, track_id))
            .or_default()
            .subscribe()
    }

    /// Очищает индикаторы активных видеоисточников.
    pub(crate) fn clear(&self) {
        self.generations.borrow_mut().clear();
        self.blocked_streams.borrow_mut().clear();
        self.subscribers.borrow_mut().clear();
        let mut live_streams = self.live_streams;
        live_streams.set(Vec::new());
    }

    fn create_renderer(
        &self,
        target_id: String,
        user_id: String,
        source: ParticipantVideoSource,
        on_dimensions: backend::ParticipantVideoDimensionsCallback,
    ) -> Result<Rc<dyn ParticipantVideoRenderer>, backend::ParticipantVideoRenderError> {
        self.backend
            .create_renderer(target_id, user_id, source.label(), on_dimensions)
    }

    fn mark_live(&self, key: ParticipantVideoKey) {
        let generation = {
            let mut generations = self.generations.borrow_mut();
            let generation = generations.entry(key.clone()).or_insert(0);
            *generation = generation.saturating_add(1);
            *generation
        };

        let mut next_streams = (self.live_streams)();
        if next_streams.iter().any(|activity| {
            activity.source == key.source
                && activity.user_id == key.user_id
                && activity.track_id == key.track_id
        }) {
            return;
        }
        next_streams.push(ParticipantVideoActivity {
            source: key.source,
            user_id: key.user_id.clone(),
            track_id: key.track_id,
        });
        let mut live_streams = self.live_streams;
        live_streams.set(next_streams);
        debug!(
            user_id = %key.user_id,
            source = key.source.label(),
            owner_scope = ?self.owner_scope,
            "marked participant video stream as live"
        );

        if key.source == ParticipantVideoSource::ScreenShare {
            debug!(user_id = %key.user_id, "keeping static screen share live until explicit stop");
            return;
        }

        let generations = self.generations.clone();
        let blocked_streams = self.blocked_streams.clone();
        let subscribers = self.subscribers.clone();
        Runtime::current().spawn(self.owner_scope, async move {
            let mut observed_generation = generation;
            loop {
                sleep_ms(PARTICIPANT_VIDEO_RELEASE_TIMEOUT_MS).await;
                let latest_generation = generations.borrow().get(&key).copied();
                match latest_generation {
                    Some(latest_generation) if latest_generation != observed_generation => {
                        observed_generation = latest_generation;
                    }
                    Some(_) => {
                        generations.borrow_mut().remove(&key);
                        blocked_streams
                            .borrow_mut()
                            .insert(key.clone(), ParticipantVideoBlockReason::AwaitingKeyFrame);
                        subscribers.borrow_mut().remove(&key);
                        let mut next_streams = live_streams();
                        let previous_len = next_streams.len();
                        next_streams.retain(|activity| {
                            activity.source != key.source
                                || activity.user_id != key.user_id
                                || activity.track_id != key.track_id
                        });
                        if next_streams.len() != previous_len {
                            live_streams.set(next_streams);
                            debug!(
                                user_id = %key.user_id,
                                source = key.source.label(),
                                "released participant video stream live marker"
                            );
                        }
                        break;
                    }
                    None => break,
                }
            }
        });
    }

    fn should_drop_blocked_frame(&self, key: &ParticipantVideoKey, key_frame: bool) -> bool {
        match self.blocked_streams.borrow().get(key).copied() {
            None => return false,
            Some(ParticipantVideoBlockReason::DepartedPresence) => return true,
            Some(ParticipantVideoBlockReason::AwaitingKeyFrame) if !key_frame => return true,
            Some(ParticipantVideoBlockReason::AwaitingKeyFrame) => {}
        }
        self.blocked_streams.borrow_mut().remove(key);
        false
    }
}

/// Внутренний ключ одного видеопотока участника.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct ParticipantVideoKey {
    source: ParticipantVideoSource,
    user_id: String,
    track_id: Uuid,
}

impl ParticipantVideoKey {
    fn new(source: ParticipantVideoSource, user_id: String, track_id: Uuid) -> Self {
        Self {
            source,
            user_id,
            track_id,
        }
    }
}

#[cfg(test)]
mod tests;
