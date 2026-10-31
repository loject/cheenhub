//! Расчет данных для сетки участников голосовой комнаты.

use std::collections::HashMap;

use cheenhub_contracts::realtime::VoiceRoomParticipant;
use dioxus::prelude::*;
use uuid::Uuid;

use super::participant_tile::VoiceParticipantTileMedia;

/// Данные одного рендеримого тайла участника.
#[derive(Clone, PartialEq)]
pub(super) struct ParticipantTileEntry {
    /// Уникальный ключ Dioxus-элемента.
    pub(super) key: String,
    /// Участник голосовой комнаты.
    pub(super) participant: VoiceRoomParticipant,
    /// Признак активной речи участника.
    pub(super) speaking: bool,
    /// Тип медиа, отображаемого в тайле.
    pub(super) media: VoiceParticipantTileMedia,
    /// UUID видеодорожки этой плитки; nil у локального preview и аватара.
    pub(super) track_id: Uuid,
}

/// Расчетные параметры flex-сетки участников.
pub(super) struct ParticipantGridLayout {
    /// Фактическое число тайлов.
    pub(super) count: usize,
    /// Число колонок.
    pub(super) columns: usize,
    /// Число строк.
    pub(super) rows: usize,
    /// CSS-переменные размера тайла.
    pub(super) style: String,
}

/// Возвращает параметры сетки для указанного количества тайлов.
pub(super) fn participant_grid_layout(tile_count: usize) -> ParticipantGridLayout {
    let count = tile_count.max(1);
    let columns = participant_grid_columns(count);
    let rows = count.div_ceil(columns);
    let focus_strip_columns = count.saturating_sub(1).max(1);
    let column_gap_rem = columns.saturating_sub(1) as f64;
    let row_gap_width_rem = rows.saturating_sub(1) as f64 * 1.777;
    let style = format!(
        "--tile-grow: 0; --tile-min: 0; --tile-columns: {columns}; --tile-rows: {rows}; --focus-strip-columns: {focus_strip_columns}; --tile-basis: min(calc((100cqw - {column_gap_rem:.3}rem) / {columns}), calc((177.777cqh - {row_gap_width_rem:.3}rem) / {rows})); --tile-max: var(--tile-basis);"
    );

    ParticipantGridLayout {
        count,
        columns,
        rows,
        style,
    }
}

/// Выбирает лучший тайл для глобальной кнопки режима отображения.
pub(super) fn preferred_focus_tile_key(tiles: &[ParticipantTileEntry]) -> Option<String> {
    tiles
        .iter()
        .find(|tile| tile.media == VoiceParticipantTileMedia::ScreenShare)
        .or_else(|| {
            tiles
                .iter()
                .find(|tile| tile.media == VoiceParticipantTileMedia::Camera)
        })
        .or_else(|| tiles.first())
        .map(|tile| tile.key.clone())
}

/// Собирает рендеримые тайлы участников с учетом доступных медиа.
pub(super) fn participant_tiles(
    participants: &[VoiceRoomParticipant],
    speaking_sessions: &[(String, Option<Uuid>)],
    camera_tracks: &[(String, Uuid)],
    screen_tracks: &[(String, Uuid)],
    current_user_id: &str,
    current_session_id: Option<Uuid>,
    local_camera_live: bool,
) -> Vec<ParticipantTileEntry> {
    let mut tiles = Vec::new();
    for participant in participants {
        let speaking = speaking_sessions.iter().any(|(user_id, session_id)| {
            user_id == &participant.user_id
                && session_id.is_none_or(|id| id.to_string() == participant.session_id)
        });
        let local_session = participant.user_id == current_user_id
            && current_session_id.is_some_and(|id| {
                participant.session_id.is_empty() || id.to_string() == participant.session_id
            });
        let track_for = |tracks: &[(String, Uuid)]| {
            tracks.iter().find_map(|(user_id, track_id)| {
                (user_id == &participant.user_id
                    && if track_id.is_nil() {
                        local_session
                    } else {
                        participant.session_id.is_empty()
                            || track_id.to_string() == participant.session_id
                    })
                .then_some(*track_id)
            })
        };
        let screen_track = track_for(screen_tracks);
        let camera_track = track_for(camera_tracks)
            .or_else(|| (local_session && local_camera_live).then_some(Uuid::nil()));

        if let Some(track_id) = screen_track {
            tiles.push(tile_entry(
                participant,
                speaking,
                VoiceParticipantTileMedia::ScreenShare,
                track_id,
            ));
        }
        if let Some(track_id) = camera_track {
            tiles.push(tile_entry(
                participant,
                speaking,
                VoiceParticipantTileMedia::Camera,
                track_id,
            ));
        }
        if screen_track.is_none() && camera_track.is_none() {
            tiles.push(tile_entry(
                participant,
                speaking,
                VoiceParticipantTileMedia::Avatar,
                Uuid::nil(),
            ));
        }
    }

    ensure_unique_tile_keys(&mut tiles);

    tiles
}

fn participant_grid_columns(count: usize) -> usize {
    match count {
        0 | 1 => 1,
        2 => 2,
        3 | 4 => 2,
        _ => ((count as f64 * 1.3125).sqrt().ceil() as usize).clamp(3, count),
    }
}

fn ensure_unique_tile_keys(tiles: &mut [ParticipantTileEntry]) {
    let mut seen_keys = HashMap::<String, usize>::new();
    for tile in tiles {
        let base_key = tile.key.clone();
        let occurrence = seen_keys.entry(base_key.clone()).or_default();
        if *occurrence > 0 {
            let duplicate_index = *occurrence;
            warn!(
                tile_key = %base_key,
                duplicate_index,
                "deduplicated voice participant tile key"
            );
            tile.key = format!("{base_key}-dup-{duplicate_index}");
        }
        *occurrence += 1;
    }
}

fn tile_entry(
    participant: &VoiceRoomParticipant,
    speaking: bool,
    media: VoiceParticipantTileMedia,
    track_id: Uuid,
) -> ParticipantTileEntry {
    let suffix = match media {
        VoiceParticipantTileMedia::Avatar => "avatar",
        VoiceParticipantTileMedia::Camera => "camera",
        VoiceParticipantTileMedia::ScreenShare => "screen",
    };

    ParticipantTileEntry {
        key: format!(
            "{}-{}-{suffix}",
            participant.user_id, participant.session_id
        ),
        participant: participant.clone(),
        speaking,
        media,
        track_id,
    }
}

#[cfg(test)]
mod tests;
