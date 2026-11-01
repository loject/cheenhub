//! Плитка пользователя, которого пригласили в голосовую комнату.

use dioxus::prelude::*;

use crate::features::app::components::avatar::UserAvatar;

use super::room_invite_tiles::{OutgoingInviteTile, OutgoingInviteTileStatus};

/// Отображает дозвон или кратковременный результат вместо фиктивного участника.
#[component]
pub(super) fn RoomInviteProgressTile(tile: OutgoingInviteTile) -> Element {
    let invites = use_context::<super::room_invite_state::RoomInviteHandle>();
    let ringing = tile.status == OutgoingInviteTileStatus::Calling;
    let (status, status_class) = match tile.status {
        OutgoingInviteTileStatus::Calling => ("Звоним…", "text-[12px] font-medium text-amber-300"),
        OutgoingInviteTileStatus::AcceptedWaiting => (
            "Принято, ожидаем подключения",
            "text-[12px] font-medium text-emerald-200",
        ),
        OutgoingInviteTileStatus::Unanswered => (
            "Не удалось дозвониться",
            "text-[12px] font-medium text-red-200",
        ),
        OutgoingInviteTileStatus::Declined => ("Отклонено", "text-[12px] font-medium text-red-200"),
        OutgoingInviteTileStatus::Cancelled => {
            ("Звонок отменён", "text-[12px] font-medium text-zinc-400")
        }
    };

    rsx! {
        article {
            "data-outgoing-invite": "{tile.invite_id}",
            class: "user-tile relative overflow-hidden rounded-[20px] border border-amber-500/20 bg-zinc-950/80 p-4 shadow-[0_18px_70px_rgba(0,0,0,.16)]",
            div { class: "room-invite-progress-content relative z-10 flex h-full min-h-full flex-col items-center justify-center gap-3 text-center",
                div { class: "room-invite-progress-avatar relative h-20 w-20 shrink-0",
                    UserAvatar {
                        nickname: tile.nickname.clone(),
                        avatar_url: tile.avatar_url.clone(),
                        class: "room-invite-progress-avatar-image flex h-20 w-20 items-center justify-center rounded-full bg-zinc-900/80 text-[26px] font-bold text-zinc-100 shadow-[0_16px_36px_rgba(0,0,0,.24)] outline outline-1 -outline-offset-1 outline-white/10".to_owned(),
                        avatar_seed: Some(tile.recipient_user_id.clone()),
                    }
                    if ringing {
                        span { class: "absolute -bottom-1 -right-1 grid h-7 w-7 place-items-center rounded-full border border-zinc-800 bg-zinc-950",
                            span { class: "h-4 w-4 animate-spin rounded-full border-2 border-zinc-700 border-t-blue-400", "aria-hidden": "true" }
                        }
                    }
                }
                div { class: "room-invite-progress-body flex min-w-0 max-w-full flex-col items-center gap-1",
                    h3 { class: "max-w-full truncate text-[13px] font-semibold text-zinc-100", "{tile.nickname}" }
                    p { class: status_class, role: "status", "{status}" }
                    if ringing {
                        button {
                            r#type: "button",
                            disabled: invites.busy(),
                            class: "mt-2 rounded-lg border border-red-500/30 bg-red-500/10 px-3 py-1.5 text-[12px] font-medium text-red-200 transition hover:bg-red-500/20 disabled:cursor-not-allowed disabled:opacity-50",
                            "aria-label": "Отменить приглашение для {tile.nickname}",
                            onclick: {
                                let handle = invites.clone();
                                let invite_id = tile.invite_id.clone();
                                move |event| {
                                    event.stop_propagation();
                                    handle.cancel(invite_id.clone());
                                }
                            },
                            "Отменить"
                        }
                    }
                }
            }
        }
    }
}
