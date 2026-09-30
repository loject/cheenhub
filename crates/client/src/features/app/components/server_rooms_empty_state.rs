//! Пустое состояние списка комнат сервера.

use dioxus::prelude::*;

/// Показывает следующий шаг, когда на сервере пока нет комнат.
#[component]
pub(super) fn ServerRoomsEmptyState(
    can_manage_rooms: bool,
    on_create: EventHandler<()>,
) -> Element {
    rsx! {
        section { class: "flex min-w-0 flex-1 items-center justify-center bg-zinc-950/35 p-6",
            div { class: "max-w-sm text-center",
                h2 { class: "text-[16px] font-semibold text-zinc-100", "Комнат пока нет" }
                p { class: "mt-2 text-[13px] leading-6 text-zinc-500",
                    if can_manage_rooms {
                        "Создай первую комнату, чтобы участникам было куда перейти."
                    } else {
                        "Владелец сервера еще не создал комнаты."
                    }
                }
                if can_manage_rooms {
                    button {
                        r#type: "button",
                        class: "mt-4 inline-flex h-10 items-center justify-center rounded-xl bg-accent px-4 text-[13px] font-semibold text-white transition hover:bg-blue-400",
                        onclick: move |_| on_create.call(()),
                        "Создать комнату"
                    }
                }
            }
        }
    }
}
