//! Выбор участников сервера для приглашения в активную голосовую комнату.

use std::collections::HashSet;

use cheenhub_contracts::realtime::{VoiceRoomInviteCandidate, VoiceRoomInviteStatus};
use dioxus::prelude::*;

use crate::features::app::components::avatar::UserAvatar;
use crate::features::realtime::RealtimeHandle;

use super::room_invite_realtime;
use super::room_invite_state::RoomInviteHandle;

/// Кнопка приглашения и раскрывающийся список участников над плавающей панелью.
#[component]
pub(super) fn RoomInvitePicker(server_id: String, room_id: String, enabled: bool) -> Element {
    let realtime = use_context::<RealtimeHandle>();
    let invites = use_context::<RoomInviteHandle>();
    let mut opened = use_signal(|| false);
    let mut loading = use_signal(|| false);
    let mut candidates = use_signal(Vec::<VoiceRoomInviteCandidate>::new);
    let mut load_error = use_signal(|| None::<String>);
    let load_realtime = realtime.clone();
    let load_server_id = server_id.clone();
    let load_room_id = room_id.clone();
    let outgoing = invites.outgoing_for_room(&server_id, &room_id);
    let pending_ids: HashSet<String> = outgoing
        .iter()
        .filter(|invite| invite.status == VoiceRoomInviteStatus::Pending)
        .map(|invite| invite.recipient_user_id.clone())
        .collect();
    let pending_invites = outgoing
        .iter()
        .filter(|invite| invite.status == VoiceRoomInviteStatus::Pending)
        .cloned()
        .collect::<Vec<_>>();

    rsx! {
        div { class: "relative",
            button {
                r#type: "button",
                disabled: !enabled,
                class: "voice-control-button group relative flex h-14 w-14 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 text-zinc-200 transition-[transform,background-color,border-color,color,opacity] duration-[180ms] hover:-translate-y-0.5 hover:border-blue-400/40 hover:bg-blue-500/15 active:scale-[0.96] disabled:cursor-not-allowed disabled:opacity-50",
                "aria-label": "Пригласить в голосовую комнату",
                "aria-expanded": opened(),
                onclick: move |_| {
                    if !enabled {
                        return;
                    }
                    if opened() {
                        opened.set(false);
                        return;
                    }
                    opened.set(true);
                    loading.set(true);
                    load_error.set(None);
                    let realtime = load_realtime.clone();
                    let server_id = load_server_id.clone();
                    let room_id = load_room_id.clone();
                    spawn(async move {
                        match room_invite_realtime::candidates(&realtime, server_id, room_id).await {
                            Ok(snapshot) => candidates.set(snapshot.candidates),
                            Err(error) => load_error.set(Some(error.to_string())),
                        }
                        loading.set(false);
                    });
                },
                span { class: "pointer-events-none absolute bottom-[calc(100%+10px)] left-1/2 -translate-x-1/2 translate-y-1 whitespace-nowrap rounded-xl border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-[12px] font-medium text-zinc-200 opacity-0 transition-[opacity,transform] duration-150 group-hover:translate-y-0 group-hover:opacity-100 group-focus-visible:translate-y-0 group-focus-visible:opacity-100", "Пригласить" }
                svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2m8-14a4 4 0 1 1-8 0 4 4 0 0 1 8 0Zm9 1v6m-3-3h6" }
                }
            }
            if opened() {
                div { class: "absolute bottom-[calc(100%+14px)] right-0 z-[60] w-80 max-w-[calc(100vw-32px)] rounded-2xl border border-zinc-700 bg-zinc-950 p-3 text-zinc-100 shadow-2xl",
                    div { class: "mb-3 flex items-center justify-between gap-2",
                        p { class: "text-[13px] font-semibold", "Пригласить в комнату" }
                        button {
                            r#type: "button",
                            class: "rounded-md px-2 py-1 text-xs text-zinc-400 hover:bg-zinc-800 hover:text-white",
                            onclick: move |_| opened.set(false),
                            "Закрыть"
                        }
                    }
                    if loading() {
                        p { class: "py-4 text-center text-xs text-zinc-400", "Загружаем участников…" }
                    } else if let Some(message) = load_error() {
                        p { role: "alert", class: "rounded-lg bg-red-500/10 p-2 text-xs text-red-200", "{message}" }
                    } else if candidates().is_empty() {
                        p { class: "py-4 text-center text-xs text-zinc-400", "Некого пригласить: остальные участники уже в комнате или недоступны." }
                    } else {
                        div { class: "max-h-72 space-y-1 overflow-y-auto",
                            for candidate in candidates() {
                                div { key: "{candidate.user_id}", class: "flex min-w-0 items-center gap-2 rounded-lg p-1.5 hover:bg-zinc-900",
                                    UserAvatar {
                                        nickname: candidate.nickname.clone(),
                                        avatar_url: candidate.avatar_url.clone(),
                                        class: "flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-zinc-800 text-xs font-bold text-zinc-100".to_owned(),
                                        avatar_seed: Some(candidate.user_id.clone()),
                                    }
                                    span { class: "min-w-0 flex-1 truncate text-[13px]", "{candidate.nickname}" }
                                    if let Some(pending_invite) = pending_invites.iter().find(|invite| invite.recipient_user_id == candidate.user_id).cloned() {
                                        span { class: "shrink-0 text-[11px] text-amber-300", "Ожидает" }
                                        button {
                                            r#type: "button",
                                            disabled: invites.busy(),
                                            class: "shrink-0 rounded-lg border border-amber-500/20 px-2 py-1.5 text-[11px] font-medium text-amber-200 hover:bg-amber-500/10 disabled:opacity-50",
                                            "aria-label": "Отменить приглашение",
                                            onclick: {
                                                let handle = invites.clone();
                                                let invite_id = pending_invite.invite_id.clone();
                                                move |_| handle.cancel(invite_id.clone())
                                            },
                                            "Отменить"
                                        }
                                    } else {
                                        button {
                                            r#type: "button",
                                            disabled: invites.busy() || pending_ids.contains(&candidate.user_id),
                                            class: "shrink-0 rounded-lg bg-blue-600 px-2.5 py-1.5 text-[11px] font-semibold text-white hover:bg-blue-500 disabled:opacity-50",
                                            onclick: {
                                                let handle = invites.clone();
                                                let recipient_id = candidate.user_id.clone();
                                                let target_server_id = server_id.clone();
                                                let target_room_id = room_id.clone();
                                                move |_| handle.send(target_server_id.clone(), target_room_id.clone(), recipient_id.clone())
                                            },
                                            "Позвать"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
