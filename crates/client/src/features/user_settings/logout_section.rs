//! User logout settings section.

use dioxus::prelude::*;

use super::page::UserSettingsSection;

/// Renders the mock sign-out action area.
#[component]
pub(crate) fn LogoutSettingsSection(
    on_select_section: EventHandler<UserSettingsSection>,
) -> Element {
    rsx! {
        div { class: "rounded-2xl border border-red-500/20 bg-red-500/10 p-4",
            h3 { class: "text-[16px] font-semibold tracking-[-0.03em] text-red-100", "Выйти из аккаунта" }
            p { class: "mt-1 max-w-xl text-[12px] leading-5 text-red-100/70", "Заверши текущий сеанс на этом устройстве." }
            div { class: "mt-4 flex gap-2",
                button { r#type: "button", class: "flex h-9 items-center justify-center rounded-xl bg-red-500 px-4 text-[12px] font-semibold text-white transition hover:bg-red-400", "Выйти" }
                button {
                    r#type: "button",
                    class: "flex h-9 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 px-4 text-[12px] font-medium text-zinc-300 transition hover:border-zinc-700 hover:bg-zinc-900 hover:text-zinc-100",
                    onclick: move |_| on_select_section.call(UserSettingsSection::Profile),
                    "Остаться"
                }
            }
        }
    }
}
