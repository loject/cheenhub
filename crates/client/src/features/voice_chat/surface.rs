//! Voice room surface component.

use cheenhub_contracts::rest::ServerRoomKind;
use dioxus::prelude::*;

use crate::features::app::components::app_shell::ActiveRoom;
use crate::features::microphone::{MicrophoneHandle, MicrophoneStatus};

use super::participant_grid::VoiceParticipantGrid;
use super::state::{VoiceConnectionHandle, VoiceConnectionState, VoiceRoomTarget};
use super::voice_controls::VoiceControls;

/// Renders one voice room surface.
#[component]
pub(crate) fn VoiceRoomSurface(server_id: String, room: ActiveRoom) -> Element {
    let voice = use_context::<VoiceConnectionHandle>();
    let microphone = use_context::<MicrophoneHandle>();
    let state = voice.state();
    let is_active_room = state.is_active_room(&server_id, &room.id);
    let participants = if is_active_room {
        state.participants().to_vec()
    } else {
        Vec::new()
    };
    let mut speaking_user_ids = if is_active_room {
        voice.speaking_user_ids()
    } else {
        Vec::new()
    };
    let microphone_live = matches!(microphone.status(), MicrophoneStatus::Live);
    if is_active_room && microphone_live && microphone.level().active {
        let current_user_id = voice.current_user_id().to_owned();
        if !speaking_user_ids
            .iter()
            .any(|user_id| user_id == &current_user_id)
        {
            speaking_user_ids.push(current_user_id);
        }
    }
    let can_join = room.kind != ServerRoomKind::Text;
    let is_busy = matches!(
        state,
        VoiceConnectionState::Connecting { .. } | VoiceConnectionState::Disconnecting { .. }
    );
    let join_label = if is_busy && is_active_room {
        "Подключаемся..."
    } else {
        "Войти в голосовую комнату"
    };

    rsx! {
        div { class: "voice-room-surface relative flex min-h-0 flex-1 flex-col",
            if is_active_room {
                VoiceParticipantGrid { participants, speaking_user_ids }
            } else {
                div { class: "voice-stage flex min-h-0 flex-1 items-center justify-center p-6 pb-[108px]",
                    div { class: "max-w-sm text-center",
                        div { class: "mx-auto grid h-16 w-16 place-items-center rounded-2xl border border-zinc-800 bg-zinc-900/80 text-zinc-500",
                            svg { class: "h-7 w-7", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19 11a7 7 0 0 1-14 0m7 8v3m-4 0h8m-4-18a3 3 0 0 0-3 3v4a3 3 0 1 0 6 0V7a3 3 0 0 0-3-3Z" }
                            }
                        }
                        h2 { class: "mt-4 text-[16px] font-semibold text-zinc-100", "{room.name}" }
                        p { class: "mt-2 text-[13px] leading-6 text-zinc-500",
                            if can_join {
                                "Подключись к комнате, чтобы видеть участников звонка."
                            } else {
                                "В этой комнате нет голосового чата."
                            }
                        }
                        if can_join {
                            button {
                                r#type: "button",
                                disabled: is_busy,
                                class: "mt-4 inline-flex h-10 items-center justify-center rounded-xl bg-accent px-4 text-[13px] font-semibold text-white transition hover:bg-blue-400 disabled:cursor-wait disabled:opacity-70",
                                onclick: {
                                    let target = VoiceRoomTarget {
                                        server_id: server_id.clone(),
                                        room_id: room.id.clone(),
                                        room_name: room.name.clone(),
                                    };
                                    move |_| voice.join(target.clone())
                                },
                                "{join_label}"
                            }
                        }
                    }
                }
            }
            VoiceControls {
                server_id: server_id.clone(),
                room_id: room.id.clone(),
            }
        }
    }
}
