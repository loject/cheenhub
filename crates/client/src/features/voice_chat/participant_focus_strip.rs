//! Горизонтальный список участников для режима фокуса.

use dioxus::prelude::*;

use crate::features::app::components::avatar::UserAvatar;

use super::participant_grid_data::FocusStripTile;
use super::participant_tile::VoiceParticipantTileMedia;

/// Выбор участника в списке режима фокуса.
#[derive(Clone, PartialEq, Eq)]
pub(super) struct FocusStripSelection {
    /// Идентификатор выбранного пользователя.
    pub(super) user_id: String,
    /// Ключ тайла, который нужно сфокусировать.
    pub(super) tile_key: String,
    /// Тип медиа выбранного тайла.
    pub(super) media: VoiceParticipantTileMedia,
}

/// Рендерит горизонтальный список участников в режиме фокуса.
#[component]
pub(super) fn ParticipantFocusStrip(
    tiles: Vec<FocusStripTile>,
    active_tile_key: Option<String>,
    on_select: EventHandler<FocusStripSelection>,
) -> Element {
    let tile_count = tiles.len();

    rsx! {
        div { class: "voice-focus-strip",
            div {
                class: "voice-focus-strip-inner",
                "data-count": "{tile_count}",
                for item in tiles {
                    button {
                        key: "{item.tile_key}",
                        r#type: "button",
                        class: if active_tile_key.as_deref() == Some(item.tile_key.as_str()) {
                            "group flex min-w-[132px] max-w-[200px] shrink-0 items-center gap-2 rounded-2xl border border-accent/50 bg-accent/15 px-2.5 py-2 text-left text-zinc-50 shadow-[0_12px_34px_rgba(59,130,246,.18)] transition-[background,border-color,color,transform,box-shadow] duration-150 hover:-translate-y-px hover:border-accent/65 hover:bg-accent/20"
                        } else {
                            "group flex min-w-[132px] max-w-[200px] shrink-0 items-center gap-2 rounded-2xl border border-zinc-800 bg-zinc-950/82 px-2.5 py-2 text-left text-zinc-300 shadow-[0_12px_34px_rgba(0,0,0,.24)] transition-[background,border-color,color,transform,box-shadow] duration-150 hover:-translate-y-px hover:border-zinc-700 hover:bg-zinc-900/92 hover:text-zinc-100"
                        },
                        "aria-current": if active_tile_key.as_deref() == Some(item.tile_key.as_str()) { "true" } else { "false" },
                        "aria-label": "Сфокусироваться на {item.nickname}",
                        onclick: {
                            let target_tile_key = item.tile_key.clone();
                            let target_user_id = item.user_id.clone();
                            let target_media = item.media;
                            move |event| {
                                event.stop_propagation();
                                on_select.call(FocusStripSelection {
                                    user_id: target_user_id.clone(),
                                    tile_key: target_tile_key.clone(),
                                    media: target_media,
                                });
                            }
                        },
                        div { class: "relative shrink-0",
                            UserAvatar {
                                nickname: item.nickname.clone(),
                                avatar_url: item.avatar_url.clone(),
                                class: "flex h-9 w-9 items-center justify-center rounded-xl border border-white/10 bg-zinc-900 text-[14px] font-bold text-zinc-100".to_owned(),
                                avatar_seed: Some(item.user_id.clone()),
                            }
                            if item.speaking {
                                span { class: "absolute -bottom-0.5 -right-0.5 h-3 w-3 rounded-full border-2 border-zinc-950 bg-emerald-400" }
                            }
                        }
                        div { class: "min-w-0 flex-1",
                            div { class: "flex min-w-0 items-center gap-1.5",
                                if item.media == VoiceParticipantTileMedia::ScreenShare {
                                    svg { class: "h-3.5 w-3.5 shrink-0 text-sky-300", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                        rect { x: "3", y: "4", width: "18", height: "12", rx: "2" }
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M8 20h8m-4-4v-9m0 0-3 3m3-3 3 3" }
                                    }
                                } else if item.media == VoiceParticipantTileMedia::Camera {
                                    svg { class: "h-3.5 w-3.5 shrink-0 text-cyan-300", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "m15 10 4.55-2.28A1 1 0 0 1 21 8.62v6.76a1 1 0 0 1-1.45.9L15 14m0-4v4m0-4a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v4a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2" }
                                    }
                                }
                                span { class: "truncate text-[13px] font-semibold", "{item.nickname}" }
                            }
                            if item.is_self {
                                div { class: "mt-0.5 text-[11px] font-medium text-zinc-500", "Вы" }
                            }
                        }
                    }
                }
            }
        }
    }
}
