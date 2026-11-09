//! Назначение владельца по email или ID через общий обработчик страницы.
//!
//! Форма хранит только ввод; запрос, результат и блокировка конкурирующих
//! операций остаются у страницы владельцев.

use dioxus::prelude::*;

/// Показывает ввод пользователя и отправляет команду назначения владельцем.
#[component]
pub(super) fn HostOwnerGrantForm(is_busy: bool, on_grant: EventHandler<String>) -> Element {
    let mut input = use_signal(String::new);

    rsx! {
        form {
            class: "rounded-[18px] border border-zinc-800 bg-zinc-950/70 p-5",
            onsubmit: move |event| {
                event.prevent_default();
                let user = input().trim().to_owned();
                if !is_busy && !user.is_empty() {
                    on_grant.call(user);
                }
            },
            h2 { class: "text-[15px] font-semibold text-zinc-100", "Выдать права владельца" }
            p { class: "mt-1.5 text-pretty text-[13px] leading-5 text-zinc-500",
                "Укажи email или ID пользователя CheenHub, чтобы назначить его владельцем хоста."
            }
            label { r#for: "host-owner-lookup", class: "mt-4 block text-[12px] font-medium text-zinc-400",
                "Email или ID пользователя"
            }
            div { class: "mt-2 flex flex-col gap-3 sm:flex-row",
                input {
                    id: "host-owner-lookup",
                    r#type: "text",
                    class: "h-11 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[13px] text-zinc-100 outline-none transition-[border-color,box-shadow] duration-150 placeholder:text-zinc-700 focus:border-accent/70 focus:ring-4 focus:ring-accent/10 disabled:cursor-not-allowed disabled:opacity-60",
                    value: "{input()}",
                    placeholder: "email или ID пользователя",
                    disabled: is_busy,
                    required: true,
                    oninput: move |event| input.set(event.value()),
                }
                button {
                    r#type: "submit",
                    class: "inline-flex min-h-11 shrink-0 items-center justify-center rounded-xl bg-accent px-5 text-[13px] font-semibold text-white transition-colors hover:bg-blue-400 focus-visible:outline-2 focus-visible:outline-accent disabled:cursor-not-allowed disabled:opacity-50",
                    disabled: is_busy || input.read().trim().is_empty(),
                    if is_busy { "Подождите..." } else { "Выдать права" }
                }
            }
        }
    }
}
