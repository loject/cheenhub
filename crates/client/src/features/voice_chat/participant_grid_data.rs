//! Расчет данных для сетки участников голосовой комнаты.

use std::collections::HashMap;

use cheenhub_contracts::realtime::VoiceRoomParticipant;
use dioxus::prelude::*;

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
}

/// Данные тайла для нижнего списка в режиме фокуса.
#[derive(Clone, PartialEq)]
pub(super) struct FocusStripTile {
    /// Идентификатор пользователя.
    pub(super) user_id: String,
    /// Отображаемое имя пользователя.
    pub(super) nickname: String,
    /// URL аватара пользователя.
    pub(super) avatar_url: Option<String>,
    /// Ключ тайла, который будет сфокусирован при выборе участника.
    pub(super) tile_key: String,
    /// Признак активной речи участника.
    pub(super) speaking: bool,
    /// Тип медиа выбранного тайла.
    pub(super) media: VoiceParticipantTileMedia,
    /// Признак текущего пользователя.
    pub(super) is_self: bool,
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
    let column_gap_rem = columns.saturating_sub(1) as f64;
    let row_gap_width_rem = rows.saturating_sub(1) as f64 * 1.777;
    let style = format!(
        "--tile-grow: 0; --tile-min: 0; --tile-columns: {columns}; --tile-rows: {rows}; --tile-basis: min(calc((100cqw - {column_gap_rem:.3}rem) / {columns}), calc((177.777cqh - {row_gap_width_rem:.3}rem) / {rows})); --tile-max: var(--tile-basis);"
    );

    ParticipantGridLayout {
        count,
        columns,
        rows,
        style,
    }
}

/// Собирает тайлы нижнего списка режима фокуса.
pub(super) fn focus_strip_tiles(
    participants: &[VoiceRoomParticipant],
    tiles: &[ParticipantTileEntry],
    current_user_id: &str,
) -> Vec<FocusStripTile> {
    let mut items = Vec::with_capacity(tiles.len());

    if let Some(participant) = participants
        .iter()
        .find(|participant| participant.user_id == current_user_id)
    {
        push_focus_strip_tiles(&mut items, participant, tiles, current_user_id);
    }

    for participant in participants {
        if participant.user_id != current_user_id {
            push_focus_strip_tiles(&mut items, participant, tiles, current_user_id);
        }
    }

    items
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
    speaking_user_ids: &[String],
    camera_user_ids: &[String],
    screen_user_ids: &[String],
    current_user_id: &str,
    local_camera_live: bool,
) -> Vec<ParticipantTileEntry> {
    let mut tiles = Vec::new();
    for participant in participants {
        let speaking = speaking_user_ids
            .iter()
            .any(|user_id| user_id == &participant.user_id);
        let screen_sharing = screen_user_ids
            .iter()
            .any(|user_id| user_id == &participant.user_id);
        let local_camera = participant.user_id == current_user_id && local_camera_live;
        let remote_camera = participant.user_id != current_user_id
            && camera_user_ids
                .iter()
                .any(|user_id| user_id == &participant.user_id);
        let camera_on = local_camera || remote_camera;

        if screen_sharing {
            tiles.push(tile_entry(
                participant,
                speaking,
                VoiceParticipantTileMedia::ScreenShare,
            ));
        }
        if camera_on {
            tiles.push(tile_entry(
                participant,
                speaking,
                VoiceParticipantTileMedia::Camera,
            ));
        }
        if !screen_sharing && !camera_on {
            tiles.push(tile_entry(
                participant,
                speaking,
                VoiceParticipantTileMedia::Avatar,
            ));
        }
    }

    ensure_unique_tile_keys(&mut tiles);

    tiles
}

fn push_focus_strip_tiles(
    items: &mut Vec<FocusStripTile>,
    participant: &VoiceRoomParticipant,
    tiles: &[ParticipantTileEntry],
    current_user_id: &str,
) {
    for tile in tiles
        .iter()
        .filter(|tile| tile.participant.user_id == participant.user_id)
    {
        items.push(FocusStripTile {
            user_id: participant.user_id.clone(),
            nickname: participant.nickname.clone(),
            avatar_url: participant.avatar_url.clone(),
            tile_key: tile.key.clone(),
            speaking: tile.speaking,
            media: tile.media,
            is_self: participant.user_id == current_user_id,
        });
    }
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
) -> ParticipantTileEntry {
    let suffix = match media {
        VoiceParticipantTileMedia::Avatar => "avatar",
        VoiceParticipantTileMedia::Camera => "camera",
        VoiceParticipantTileMedia::ScreenShare => "screen",
    };

    ParticipantTileEntry {
        key: format!("{}-{suffix}", participant.user_id),
        participant: participant.clone(),
        speaking,
        media,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_strip_includes_all_tiles_for_participant() {
        let participants = vec![
            participant("current-user", "Текущий"),
            participant("other-user", "Сосед"),
        ];
        let tiles = participant_tiles(
            &participants,
            &[],
            &[],
            &["current-user".to_owned()],
            "current-user",
            true,
        );

        let focus_tiles = focus_strip_tiles(&participants, &tiles, "current-user");

        assert_eq!(focus_tiles.len(), 3);
        assert_eq!(focus_tiles[0].user_id, "current-user");
        assert_eq!(focus_tiles[0].tile_key, "current-user-screen");
        assert_eq!(focus_tiles[0].media, VoiceParticipantTileMedia::ScreenShare);
        assert_eq!(focus_tiles[1].user_id, "current-user");
        assert_eq!(focus_tiles[1].tile_key, "current-user-camera");
        assert_eq!(focus_tiles[1].media, VoiceParticipantTileMedia::Camera);
        assert_eq!(focus_tiles[2].user_id, "other-user");
        assert_eq!(focus_tiles[2].tile_key, "other-user-avatar");
        assert_eq!(focus_tiles[2].media, VoiceParticipantTileMedia::Avatar);
    }

    fn participant(user_id: &str, nickname: &str) -> VoiceRoomParticipant {
        VoiceRoomParticipant {
            user_id: user_id.to_owned(),
            nickname: nickname.to_owned(),
            avatar_url: None,
            joined_at: "2026-06-19T00:00:00Z".to_owned(),
        }
    }
}
