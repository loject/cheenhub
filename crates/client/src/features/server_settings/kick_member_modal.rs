//! Server member kick confirmation modal.

use dioxus::prelude::*;

use super::members_data::KickMemberTarget;
use crate::features::app::components::modal::Modal;

/// Renders a member kick confirmation with a rejoin-block duration picker.
#[component]
pub(super) fn KickMemberModal(
    member: KickMemberTarget,
    is_busy: bool,
    error: String,
    on_cancel: EventHandler<()>,
    on_confirm: EventHandler<Option<u64>>,
) -> Element {
    let mut duration = use_signal(|| "86400".to_owned());
    let member_name = member.name.clone();
    let disabled = is_busy;

    rsx! {
        Modal {
            title: "Исключить участника",
            on_close: move |_| {
                if !disabled {
                    on_cancel.call(());
                }
            },
            div { class: "space-y-4",
                div { class: "rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-3",
                    p { class: "text-[13px] font-medium text-red-100", "{member_name}" }
                    p { class: "mt-1 text-[12px] leading-5 text-red-200",
                        "Участник покинет сервер и не сможет вернуться по инвайту до окончания выбранного срока."
                    }
                }

                label { class: "block",
                    span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "Срок исключения" }
                    select {
                        name: "member-kick-duration",
                        value: duration(),
                        disabled,
                        onchange: move |event| duration.set(event.value()),
                        class: "h-11 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[14px] text-zinc-100 outline-none transition focus:border-accent/70 focus:ring-4 focus:ring-accent/10 disabled:cursor-not-allowed disabled:opacity-60",
                        option { value: "none", "Можно вернуться сразу" }
                        option { value: "3600", "1 час" }
                        option { value: "86400", "1 день" }
                        option { value: "604800", "7 дней" }
                        option { value: "2592000", "30 дней" }
                    }
                }

                if !error.is_empty() {
                    p { class: "rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-2 text-[12px] leading-5 text-red-200",
                        "{error}"
                    }
                }

                div { class: "flex justify-end gap-2 pt-1",
                    button {
                        r#type: "button",
                        disabled,
                        class: "flex h-10 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 px-4 text-[13px] font-medium text-zinc-300 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-zinc-700 hover:bg-zinc-900 hover:text-zinc-100 disabled:cursor-not-allowed disabled:opacity-60",
                        onclick: move |_| on_cancel.call(()),
                        "Отмена"
                    }
                    button {
                        r#type: "button",
                        disabled,
                        class: "flex h-10 items-center justify-center rounded-xl border border-red-500/30 bg-red-500/15 px-4 text-[13px] font-semibold text-red-100 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-red-500/45 hover:bg-red-500/20 disabled:cursor-not-allowed disabled:opacity-60",
                        onclick: move |_| {
                            if disabled {
                                return;
                            }
                            on_confirm.call(duration_seconds(&duration()));
                        },
                        if disabled {
                            "Исключаем..."
                        } else {
                            "Исключить"
                        }
                    }
                }
            }
        }
    }
}

fn duration_seconds(value: &str) -> Option<u64> {
    if value == "none" {
        return None;
    }

    value.parse::<u64>().ok()
}
