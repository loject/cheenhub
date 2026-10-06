//! Форма выдачи прав владельца хоста.
//!
//! Пользователь вводит email или идентификатор: сервер разбирает формат сам,
//! поэтому клиент не пытается определить пользователя заранее. Успешный ответ
//! содержит обновлённый список владельцев, который форма возвращает наружу.

use cheenhub_contracts::rest::HostOwnersResponse;
use dioxus::prelude::*;

use super::api;

/// Показывает поле ввода и состояние последней операции выдачи прав.
#[component]
pub(super) fn HostOwnerGrantForm(on_granted: EventHandler<HostOwnersResponse>) -> Element {
    let mut input = use_signal(String::new);
    let mut is_granting = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut granted = use_signal(|| None::<String>);

    // Пустое поле и повторное нажатие во время запроса не отправляют ничего:
    // сервер вернул бы ошибку с тем же смыслом, но без понятного объяснения.
    let submit = use_callback(move |_| {
        let request_user = input().trim().to_owned();
        if request_user.is_empty() || is_granting() {
            return;
        }

        is_granting.set(true);
        error.set(None);
        granted.set(None);
        spawn(async move {
            match api::grant_owner(request_user.clone()).await {
                Ok(owners) => {
                    info!(%request_user, "granted host owner rights from host settings");
                    granted.set(Some(
                        "Права владельца хоста выданы. Пользователь увидит настройки хоста, как только войдёт в CheenHub.".to_owned(),
                    ));
                    input.set(String::new());
                    is_granting.set(false);
                    on_granted.call(owners);
                }
                Err(failure) => {
                    warn!(%request_user, message = %failure.message(), "failed to grant host owner rights");
                    error.set(Some(failure.message().to_owned()));
                    is_granting.set(false);
                }
            }
        });
    });

    rsx! {
        div { class: "rounded-[18px] border border-zinc-800 bg-zinc-950/70 p-5",
            h2 { class: "text-[15px] font-semibold text-zinc-100", "Выдать права владельца" }
            p { class: "mt-1.5 text-pretty text-[13px] leading-5 text-zinc-500",
                "Введи email или идентификатор пользователя CheenHub. Он сразу получит доступ ко всем настройкам этого хоста."
            }
            div { class: "mt-4 flex flex-col gap-3 sm:flex-row",
                input {
                    r#type: "text",
                    class: "h-11 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[13px] text-zinc-100 outline-none transition-[border-color,box-shadow] duration-150 placeholder:text-zinc-700 focus:border-accent/70 focus:ring-4 focus:ring-accent/10 disabled:cursor-not-allowed disabled:opacity-60",
                    value: "{input()}",
                    placeholder: "email или идентификатор пользователя",
                    disabled: is_granting(),
                    oninput: move |event| input.set(event.value()),
                    onkeydown: move |event| {
                        if event.key() == Key::Enter {
                            event.prevent_default();
                            submit.call(());
                        }
                    },
                }
                button {
                    r#type: "button",
                    class: grant_button_class(is_granting()),
                    disabled: is_granting(),
                    onclick: move |_| submit.call(()),
                    if is_granting() { "Выдаём..." } else { "Выдать права" }
                }
            }
            if let Some(message) = error() {
                p { class: "mt-3 rounded-xl bg-red-400/10 px-3 py-2 text-pretty text-[12px] leading-5 text-red-200", "{message}" }
            }
            if let Some(message) = granted() {
                p { class: "mt-3 rounded-xl bg-emerald-400/10 px-3 py-2 text-pretty text-[12px] leading-5 text-emerald-200", "{message}" }
            }
        }
    }
}

/// Класс кнопки выдачи прав в зависимости от состояния операции.
fn grant_button_class(granting: bool) -> &'static str {
    if granting {
        "inline-flex min-h-11 cursor-not-allowed items-center justify-center rounded-xl bg-zinc-800 px-5 text-[13px] font-semibold text-zinc-500"
    } else {
        "inline-flex min-h-11 items-center justify-center rounded-xl bg-accent px-5 text-[13px] font-semibold text-white shadow-[0_8px_28px_rgba(59,130,246,0.18)] transition-[background-color,transform] duration-150 hover:bg-blue-400 active:scale-[0.97]"
    }
}
