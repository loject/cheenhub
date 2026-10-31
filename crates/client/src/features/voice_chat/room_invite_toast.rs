//! Уведомление о приглашении в голосовую комнату сервера.

use cheenhub_contracts::realtime::VoiceRoomInviteSnapshot;
use dioxus::prelude::*;

use crate::features::app::components::avatar::UserAvatar;

/// Действия над входящим приглашением.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RoomInviteToastAction {
    /// Принять и войти в голосовую комнату на этом устройстве.
    Accept,
    /// Отклонить с серверным ответом отправителю.
    Decline,
    /// Скрыть только локально, оставив приглашение ожидающим.
    Ignore,
}

/// Показывает отправителя, сервер, комнату и действия в глобальном toast.
#[component]
pub(super) fn RoomInviteToast(
    invite: VoiceRoomInviteSnapshot,
    busy: bool,
    on_action: EventHandler<RoomInviteToastAction>,
) -> Element {
    let nickname = invite.caller_nickname.clone();
    let server_name = invite.server_name.clone();
    let room_name = invite.room_name.clone();
    let accept = on_action;
    let decline = on_action;
    let ignore = on_action;
    rsx! {
        div { class: "flex min-w-0 flex-1 flex-col gap-3",
            div { class: "flex items-center gap-3",
                UserAvatar {
                    nickname: nickname.clone(),
                    avatar_url: invite.caller_avatar_url.clone(),
                    class: "flex h-11 w-11 shrink-0 items-center justify-center rounded-full bg-zinc-800 text-sm font-bold text-zinc-50".to_owned(),
                    avatar_seed: Some(invite.caller_user_id.clone()),
                }
                div { class: "min-w-0 flex-1",
                    p { class: "text-[12px] font-semibold leading-4 text-blue-300", "Приглашение в голосовую" }
                    p { class: "truncate text-[14px] font-semibold leading-5 text-zinc-100", "{nickname}" }
                    p { class: "text-[12px] leading-4 text-zinc-400", "{server_name} · {room_name}" }
                }
            }
            div { class: "grid grid-cols-3 gap-2",
                button {
                    r#type: "button",
                    disabled: busy,
                    class: "min-h-9 rounded-lg bg-blue-600 px-2 text-[12px] font-semibold text-white transition hover:bg-blue-500 disabled:opacity-60",
                    onclick: move |_| accept.call(RoomInviteToastAction::Accept),
                    "Принять"
                }
                button {
                    r#type: "button",
                    disabled: busy,
                    class: "min-h-9 rounded-lg bg-red-500/15 px-2 text-[12px] font-semibold text-red-200 transition hover:bg-red-500/25 disabled:opacity-60",
                    onclick: move |_| decline.call(RoomInviteToastAction::Decline),
                    "Отклонить"
                }
                button {
                    r#type: "button",
                    class: "min-h-9 rounded-lg border border-white/10 px-2 text-[12px] font-medium text-zinc-300 transition hover:bg-white/5",
                    onclick: move |_| ignore.call(RoomInviteToastAction::Ignore),
                    "Игнорировать"
                }
            }
        }
    }
}
