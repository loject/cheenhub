//! Подтверждение блокировки пользователя вне шапки личного диалога.

use dioxus::prelude::*;

use crate::features::app::components::modal::Modal;
use crate::features::toast::ToastHandle;

use super::api;

/// Подтверждает блокировку и сообщает странице об изменении списка контактов.
///
/// Вызывающая страница размещает компонент вне шапки с backdrop-фильтром.
#[component]
pub(crate) fn PrivacyBlockModal(
    user_id: String,
    nickname: String,
    on_close: EventHandler<()>,
    on_changed: EventHandler<()>,
) -> Element {
    let toast = use_context::<ToastHandle>();
    let mut busy = use_signal(|| false);

    rsx! {
        Modal {
            title: "Заблокировать пользователя",
            close_disabled: busy(),
            on_close: move |_| { if !busy() { on_close.call(()); } },
            p { class: "text-[14px] font-medium text-zinc-100", "Заблокировать {nickname}?" }
            p { class: "mt-3 text-[13px] leading-6 text-zinc-400",
                "Дружба и заявки будут удалены. Личные сообщения, звонки и приглашения станут недоступны в обе стороны. Текущий звонок завершится, история переписки сохранится."
            }
            div { class: "mt-5 flex flex-col gap-2 sm:flex-row sm:justify-end",
                button {
                    r#type: "button",
                    disabled: busy(),
                    class: "h-10 rounded-xl border border-zinc-700 px-4 text-[12px] text-zinc-200 transition hover:bg-zinc-900 disabled:opacity-50",
                    onclick: move |_| on_close.call(()),
                    "Отмена"
                }
                button {
                    r#type: "button",
                    disabled: busy(),
                    class: "flex h-10 items-center justify-center gap-2 rounded-xl border border-red-500/30 bg-red-500/15 px-4 text-[12px] font-semibold text-red-200 transition hover:bg-red-500/25 disabled:opacity-50",
                    onclick: move |_| {
                        if busy() { return; }
                        busy.set(true);
                        let blocked_id = user_id.clone();
                        spawn(async move {
                            info!(user_id = %blocked_id, "blocking user from direct message dialog");
                            match api::block_user(&blocked_id).await {
                                Ok(()) => {
                                    info!(user_id = %blocked_id, "user blocked from direct message dialog");
                                    toast.success("Пользователь заблокирован.");
                                    on_close.call(());
                                    on_changed.call(());
                                }
                                Err(error) => {
                                    warn!(user_id = %blocked_id, %error, "failed to block direct message peer");
                                    toast.error(error);
                                    busy.set(false);
                                }
                            }
                        });
                    },
                    if busy() {
                        span { class: "size-4 animate-spin rounded-full border-2 border-red-200/30 border-t-red-200", "aria-hidden": "true" }
                        "Блокируем…"
                    } else {
                        "Заблокировать"
                    }
                }
            }
        }
    }
}
