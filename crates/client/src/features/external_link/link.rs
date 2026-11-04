//! Подтверждение перехода с локальным доверием к hostname.

use super::{safety, storage};
use crate::features::app::{components::modal::Modal, current_user::CurrentUserContext};
use dioxus::prelude::*;

/// Отображает внешнюю ссылку и запрашивает согласие перед первым переходом.
#[component]
pub(crate) fn ExternalLink(url: String, children: Element) -> Element {
    let mut pending = use_signal(|| false);
    let mut remember = use_signal(|| false);
    let user = try_use_context::<CurrentUserContext>();
    let Some(parsed) = safety::checked(&url) else {
        debug!("external link rejected");
        return rsx! { span { title: "{url}", {children} } };
    };
    let normalized = parsed.to_string();
    let is_cheenhub = parsed.origin().ascii_serialization() == "https://cheenhub.ru";
    let hostname = parsed
        .host_str()
        .unwrap_or_default()
        .trim_end_matches('.')
        .to_owned();
    let user_id = user.map(|context| context.require_user().id);
    let click_url = normalized.clone();
    let click_host = hostname.clone();
    let click_user = user_id.clone();
    let confirm_url = normalized.clone();
    let confirm_host = hostname.clone();
    let trusted = is_cheenhub
        || user_id
            .as_ref()
            .is_some_and(|id| storage::contains(id, &hostname));
    rsx! {
        if trusted { a {
            href: "{normalized}", target: if is_cheenhub { "_self" } else { "_blank" }, rel: "noopener noreferrer",
            class: "text-blue-500 underline dark:text-blue-300",
            onclick: move |event| {
                event.stop_propagation();
                let valid = safety::checked(&click_url).is_some();
                if !valid || !(is_cheenhub || click_user.as_ref().is_some_and(|id| storage::contains(id, &click_host))) {
                    event.prevent_default();
                    if valid { remember.set(false); pending.set(true); }
                }
            },
            onkeydown: move |event| event.stop_propagation(),
            {children}
        } } else { button {
            r#type: "button", class: "text-left text-blue-500 underline dark:text-blue-300",
            onclick: move |event| { event.stop_propagation(); remember.set(false); pending.set(true); },
            onkeydown: move |event| event.stop_propagation(),
            {children}
        } }
        if pending() {
            Modal { title: "Переход по внешней ссылке", on_close: move |_| pending.set(false),
                p { class: "text-sm text-zinc-300", "Вы собираетесь открыть внешний сайт:" }
                p { class: "mt-3 break-all rounded-xl bg-zinc-900 p-3 text-sm text-zinc-200", "{normalized}" }
                p { class: "mt-2 break-all text-sm font-semibold text-blue-300", "{hostname}" }
                label { class: "mt-4 flex items-center gap-2 text-sm text-zinc-300",
                    input { r#type: "checkbox", checked: remember(), onchange: move |event| remember.set(event.checked()) }
                    "Больше не спрашивать для этого домена"
                }
                div { class: "mt-5 flex justify-end gap-2",
                    button { r#type: "button", class: "rounded-xl px-4 py-2 text-sm text-zinc-300 hover:bg-zinc-800", onclick: move |_| pending.set(false), "Отмена" }
                    a { href: "{confirm_url}", target: "_blank", rel: "noopener noreferrer", class: "rounded-xl bg-blue-600 px-4 py-2 text-sm font-semibold text-white hover:bg-blue-500",
                        onclick: move |event| {
                            event.stop_propagation();
                            if safety::checked(&confirm_url).is_none() { event.prevent_default(); return; }
                            if remember() && let Some(id) = &user_id { storage::trust(id, &confirm_host); }
                        },
                        "Открыть ссылку"
                    }
                }
            }
        }
    }
}
