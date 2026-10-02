//! Снимок активности голосового чата для мониторинга хоста.

use crate::state::AppState;

/// Текущие показатели активности голосового чата на хосте.
///
/// Правила подсчёта принадлежат фиче `voice_chat`: мониторинг хоста выступает
/// потребителем готового снимка и не пересчитывает активность самостоятельно.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct VoiceActivitySnapshot {
    /// Число активных голосовых подключений: комнаты и личные звонки вместе.
    pub(crate) voice_connections: u32,
    /// Число активных видеоисточников: камера и экран считаются раздельно.
    pub(crate) video_sources: u32,
}

/// Возвращает текущие показатели активности голосового чата.
///
/// Снимок всегда доступен: он строится из памяти процесса и не зависит от базы
/// данных, поэтому недоступность истории не влияет на текущие числа.
pub(crate) async fn activity_snapshot(state: &AppState) -> VoiceActivitySnapshot {
    let store = &state.voice_presence_store;
    let voice_connections = store.active_voice_connection_count().await;
    let video_sources = store.active_video_source_count().await;

    tracing::debug!(
        voice_connections,
        video_sources,
        "collected voice activity snapshot"
    );

    VoiceActivitySnapshot {
        voice_connections: u32::try_from(voice_connections).unwrap_or(u32::MAX),
        video_sources: u32::try_from(video_sources).unwrap_or(u32::MAX),
    }
}
