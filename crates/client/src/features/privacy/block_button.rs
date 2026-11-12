//! Подтверждаемая блокировка собеседника из личного диалога.

use dioxus::prelude::*;

use crate::features::toast::ToastHandle;

use super::api;

/// Предлагает блокировку с явным подтверждением необратимого удаления дружбы.
#[component]
pub(crate) fn PrivacyBlockButton(
    user_id: String,
    nickname: String,
    compact: bool,
    on_changed: EventHandler<()>,
) -> Element {
    let toast = use_context::<ToastHandle>();
    let mut confirming = use_signal(|| false);
    let mut busy = use_signal(|| false);

    rsx! {
        button {
            r#type: "button",
            class: if compact {
                "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-red-500/20 bg-red-500/5 text-red-300 transition hover:bg-red-500/15"
            } else {
                "flex w-full items-center gap-2 rounded-xl px-3 py-2.5 text-left text-[13px] text-red-300 hover:bg-red-500/10"
            },
            "aria-label": "Заблокировать {nickname}",
            title: "Заблокировать {nickname}",
            onclick: move |_| confirming.set(true),
            svg { class: "h-4 w-4 shrink-0", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24", "aria-hidden": "true",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M18.36 6.64a9 9 0 1 1-11.72 11.72M5.64 5.64l12.72 12.72M21 3 3 21" }
            }
            if !compact { "Заблокировать" }
        }
        if confirming() {
            div { class: "fixed inset-0 z-[1500] flex items-center justify-center bg-black/75 px-4 py-6 backdrop-blur-sm",
                section {
                    role: "dialog",
                    "aria-modal": "true",
                    "aria-label": "Подтвердить блокировку",
                    class: "w-full max-w-[430px] rounded-2xl border border-zinc-800 bg-zinc-950 p-5 shadow-2xl",
                    h3 { class: "text-[16px] font-semibold text-zinc-50", "Заблокировать {nickname}?" }
                    p { class: "mt-3 text-[13px] leading-5 text-zinc-400",
                        "Дружба и заявки будут удалены. Личные сообщения, звонки и приглашения станут недоступны в обе стороны, текущий звонок завершится. История переписки сохранится."
                    }
                    div { class: "mt-5 flex flex-col gap-2 sm:flex-row sm:justify-end",
                        button {
                            r#type: "button",
                            disabled: busy(),
                            class: "h-10 rounded-xl border border-zinc-700 px-4 text-[12px] text-zinc-200 hover:bg-zinc-900 disabled:opacity-50",
                            onclick: move |_| confirming.set(false),
                            "Отмена"
                        }
                        button {
                            r#type: "button",
                            disabled: busy(),
                            class: "h-10 rounded-xl border border-red-500/30 bg-red-500/15 px-4 text-[12px] font-semibold text-red-200 hover:bg-red-500/25 disabled:opacity-50",
                            onclick: move |_| {
                                if busy() { return; }
                                busy.set(true);
                                let user_id = user_id.clone();
                                spawn(async move {
                                    match api::block_user(&user_id).await {
                                        Ok(()) => {
                                            toast.success("Пользователь заблокирован.");
                                            confirming.set(false);
                                            on_changed.call(());
                                        }
                                        Err(error) => toast.error(error),
                                    }
                                    busy.set(false);
                                });
                            },
                            if busy() { "Блокируем..." } else { "Заблокировать" }
                        }
                    }
                }
            }
        }
    }
}
