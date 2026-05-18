//! Server role save bar.

use dioxus::prelude::*;

use super::roles_data::save_bar_class;

/// Renders the sticky server role save bar.
#[component]
pub(super) fn RoleSaveBar(
    dirty: bool,
    save_error: String,
    is_saving: bool,
    can_save: bool,
    on_reset: EventHandler<()>,
    on_save: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: save_bar_class(dirty),
            div { class: "role-save-bar-inner mx-auto grid max-w-3xl items-center gap-3 rounded-2xl border border-zinc-800 bg-zinc-950/95 p-3 text-center shadow-[0_18px_50px_rgba(0,0,0,0.45)] backdrop-blur-xl sm:flex sm:justify-between sm:text-left",
                div { class: "flex min-h-10 min-w-0 flex-1 flex-col justify-center",
                    div { class: "text-sm font-semibold text-zinc-50", "Есть несохраненные изменения" }
                    if save_error.is_empty() {
                        div { class: "text-xs text-zinc-500", "Сохраните или отмените изменения роли." }
                    } else {
                        div { class: "text-xs text-red-200", "{save_error}" }
                    }
                }
                div { class: "flex shrink-0 items-center justify-center gap-2 sm:justify-end",
                    button {
                        r#type: "button",
                        class: "rounded-xl border border-zinc-800 bg-zinc-900 px-3 py-2 text-[13px] font-medium text-zinc-300 transition hover:border-zinc-700 hover:text-zinc-100",
                        onclick: move |_| on_reset.call(()),
                        "Отменить"
                    }
                    button {
                        r#type: "button",
                        disabled: is_saving || !can_save,
                        class: save_button_class(is_saving || !can_save),
                        onclick: move |_| {
                            if can_save && !is_saving {
                                on_save.call(());
                            }
                        },
                        if is_saving { "Сохраняем" } else { "Сохранить" }
                    }
                }
            }
        }
    }
}

fn save_button_class(disabled: bool) -> &'static str {
    if disabled {
        "rounded-xl bg-accent/45 px-3 py-2 text-[13px] font-semibold text-white/70 shadow-[0_0_0_1px_rgba(59,130,246,0.18)]"
    } else {
        "rounded-xl bg-accent px-3 py-2 text-[13px] font-semibold text-white shadow-[0_0_0_1px_rgba(59,130,246,0.3),0_4px_18px_rgba(59,130,246,0.16)] transition hover:bg-blue-400"
    }
}
