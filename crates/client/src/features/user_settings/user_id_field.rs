//! Поле идентификатора пользователя с копированием в буфер обмена.

use dioxus::prelude::*;

use crate::features::clipboard::copy_text;
use crate::features::runtime::sleep_ms;
use crate::features::toast::ToastHandle;

use super::styles::input_class;

/// Показывает постоянный идентификатор аккаунта и копирует его по нажатию.
///
/// Идентификатор неизменяем, поэтому поле доступно только для чтения, а копирование
/// выполняется в системный буфер обмена текущей платформы.
#[component]
pub(crate) fn UserIdField(user_id: String) -> Element {
    let toast = use_context::<ToastHandle>();
    let mut is_copied = use_signal(|| false);
    let mut copy_generation = use_signal(|| 0_u64);

    rsx! {
        div { class: "block",
            span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "Идентификатор" }
            div { class: "flex gap-2",
                input {
                    r#type: "text",
                    value: "{user_id}",
                    readonly: true,
                    class: "{input_class()} min-w-0 flex-1 font-mono text-[12px]",
                }
                button {
                    r#type: "button",
                    class: "flex h-10 shrink-0 items-center gap-2 rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[12px] font-medium text-zinc-300 transition hover:border-accent/35 hover:bg-accent/10 hover:text-blue-100 disabled:cursor-wait disabled:opacity-60",
                    "aria-label": if is_copied() { "Идентификатор скопирован" } else { "Скопировать идентификатор" },
                    onclick: move |_| {
                        if is_copied() {
                            return;
                        }

                        let user_id_to_copy = user_id.clone();
                        spawn(async move {
                            match copy_text(user_id_to_copy).await {
                                Ok(()) => {
                                    let next_generation = copy_generation() + 1;
                                    copy_generation.set(next_generation);
                                    is_copied.set(true);
                                    toast.success("Идентификатор скопирован.");
                                    info!("copied current user id in profile settings");
                                    sleep_ms(1400).await;

                                    if copy_generation() == next_generation {
                                        is_copied.set(false);
                                    }
                                }
                                Err(error) => {
                                    warn!(%error, "failed to copy current user id in profile settings");
                                    toast.error(error);
                                }
                            }
                        });
                    },
                    if is_copied() { "Скопировано" } else { "Копировать" }
                }
            }
            p { class: "mt-1.5 text-[11px] leading-4 text-zinc-500",
                "Нужен для обращения в поддержку и для проверки аккаунта."
            }
        }
    }
}
