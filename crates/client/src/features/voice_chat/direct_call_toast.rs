//! Компактное содержимое глобального toast входящего личного звонка.

use cheenhub_contracts::realtime::DirectCallSnapshot;
use dioxus::prelude::*;

use crate::features::app::components::avatar::UserAvatar;

/// Действия пользователя над входящим личным звонком.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DirectCallToastAction {
    /// Принять звонок и подключиться к собеседнику.
    Accept,
    /// Отклонить звонок с отправкой ответа собеседнику.
    Decline,
    /// Скрыть предложение звонка только на этом устройстве.
    Ignore,
}

/// Показывает звонящего и действия внутри общего toast-контейнера.
#[component]
pub(super) fn DirectCallToast(
    call: DirectCallSnapshot,
    busy: bool,
    error: Option<String>,
    on_action: EventHandler<DirectCallToastAction>,
) -> Element {
    let caller_nickname = call.caller_nickname.clone();
    let accept_action = on_action;
    let decline_action = on_action;
    let ignore_action = on_action;

    rsx! {
        div { class: "flex min-w-0 flex-1 flex-col gap-3",
            div { class: "flex items-center gap-3",
                UserAvatar {
                    nickname: caller_nickname.clone(),
                    avatar_url: call.caller_avatar_url.clone(),
                    class: "flex h-11 w-11 shrink-0 items-center justify-center rounded-full bg-zinc-800 text-sm font-bold text-zinc-50".to_owned(),
                    avatar_seed: Some(call.caller_user_id.clone()),
                }
                div { class: "min-w-0 flex-1",
                    p { class: "text-[12px] font-semibold leading-4 text-blue-300", "Входящий звонок" }
                    p { class: "truncate text-[14px] font-semibold leading-5 text-zinc-100", "{caller_nickname}" }
                    p { class: "text-[12px] leading-4 text-zinc-400", "Хочет поговорить с тобой" }
                }
            }
            if let Some(message) = error {
                p {
                    role: "alert",
                    class: "rounded-lg bg-red-500/10 px-2.5 py-2 text-[12px] leading-4 text-red-200",
                    "{message}"
                }
            }
            div { class: "grid grid-cols-3 gap-2",
                button {
                    r#type: "button",
                    disabled: busy,
                    class: "min-h-9 rounded-lg bg-blue-600 px-2 text-[12px] font-semibold text-white transition hover:bg-blue-500 disabled:cursor-wait disabled:opacity-60",
                    onclick: move |_| accept_action.call(DirectCallToastAction::Accept),
                    "Принять"
                }
                button {
                    r#type: "button",
                    disabled: busy,
                    class: "min-h-9 rounded-lg bg-red-500/15 px-2 text-[12px] font-semibold text-red-200 transition hover:bg-red-500/25 disabled:cursor-wait disabled:opacity-60",
                    onclick: move |_| decline_action.call(DirectCallToastAction::Decline),
                    "Отклонить"
                }
                button {
                    r#type: "button",
                    class: "min-h-9 rounded-lg border border-white/10 px-2 text-[12px] font-medium text-zinc-300 transition hover:bg-white/5 hover:text-zinc-100",
                    onclick: move |_| ignore_action.call(DirectCallToastAction::Ignore),
                    "Игнорировать"
                }
            }
        }
    }
}
