//! Представление постоянного уведомления об обновлении и его действий.

use super::{Toast, ToastPayload};
use crate::features::toast::update_available::UpdateAvailableToast;
use dioxus::prelude::*;

/// Отрисовывает обновляемое уведомление с действиями и текущим прогрессом.
pub(super) fn render_update_available_toast(
    toast: Toast,
    update: UpdateAvailableToast,
    mut toasts: Signal<Vec<Toast>>,
) -> Element {
    let busy = update.progress.is_some();
    let on_install = update.on_install.clone();
    let on_quick_dismiss = update.on_quick_dismiss.clone();
    let on_defer = update.on_defer.clone();
    let selected_deferral_value = update.selected_deferral_value.clone();

    rsx! {
        article {
            key: "{toast.id}",
            role: toast.kind.role(),
            "aria-live": toast.kind.live_region(),
            class: update_toast_class(toast.exiting),
            div { class: "flex items-start gap-2.5 px-4 pb-3 pt-3.5",
                div { class: "mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center",
                    span { class: "h-2 w-2 rounded-full {toast.kind.accent_class()} shadow-[0_0_12px_rgba(96,165,250,0.45)]" }
                }
                div { class: "min-w-0 flex-1 space-y-0.5",
                    p { class: "text-[12px] font-semibold leading-4 text-zinc-100", if let Some(progress) = &update.progress { "{progress.title}" } else { "{toast.kind.label()}" } }
                    p { class: "break-words text-[13px] font-medium leading-5 text-zinc-300",
                        "CheenHub {update.current_version} → {update.update_version}"
                    }
                    p { class: "text-[11px] leading-4 text-zinc-500",
                        if let Some(title) = update.title.as_ref() {
                            "{title}"
                        } else {
                            "На GitHub опубликован новый релиз."
                        }
                    }
                }
                if !busy { button {
                    r#type: "button",
                    "aria-label": "Скрыть уведомление об обновлении на пять минут",
                    class: "flex h-7 w-7 shrink-0 items-center justify-center rounded-md text-[18px] leading-none text-zinc-500 transition hover:bg-white/5 hover:text-zinc-100",
                    onclick: move |_| {
                        (on_quick_dismiss.as_ref())();
                        super::super::timer::begin_dismiss_toast(&mut toasts, toast.id);
                    },
                    "×"
                } }
            }
            if let Some(progress) = &update.progress {
                div { class: "px-4 pb-3",
                    div {
                        class: "application-update-progress-track",
                        role: "progressbar",
                        aria_label: "{progress.title}",
                        aria_valuemin: "0",
                        aria_valuemax: "100",
                        aria_valuenow: progress.percentage.map(|value| format!("{value:.0}")),
                        div {
                            class: if progress.percentage.is_some() { "application-update-progress-fill" } else { "application-update-progress-fill application-update-progress-indeterminate" },
                            style: format!("width: {}%;", progress.percentage.unwrap_or(35.0)),
                        }
                    }
                    if let Some(percentage) = progress.percentage {
                        p { class: "mt-2 text-[12px] font-semibold text-blue-100", style: "font-variant-numeric: tabular-nums;", "{percentage:.0}%" }
                    }
                    p { class: "mt-2 text-[12px] leading-5 text-zinc-300", style: "font-variant-numeric: tabular-nums;", "{progress.detail}" }
                }
            }
            div { class: "grid gap-2 border-t border-white/[0.06] bg-black/10 px-3 py-2.5",
                button {
                    r#type: "button",
                    disabled: update.primary_disabled,
                    class: update_primary_button_class(update.primary_disabled),
                    onclick: move |_| (on_install.as_ref())(),
                    "{update.primary_label}"
                }
                if !busy { div { class: "grid grid-cols-[1fr_auto] gap-2",
                    div { class: "relative min-w-0",
                        select {
                            value: "{update.selected_deferral_value}",
                            class: "h-9 w-full appearance-none rounded-lg border border-white/10 bg-zinc-900/80 px-3 pr-8 text-[12px] font-medium text-zinc-200 outline-none transition hover:border-white/15 focus:border-blue-400/50 focus:ring-2 focus:ring-blue-400/10",
                            style: "color-scheme: dark;",
                            onchange: move |event| set_update_deferral_value(&mut toasts, toast.id, event.value()),
                            for option in update.deferral_options.iter() {
                                option {
                                    value: "{option.value}",
                                    selected: option.value == update.selected_deferral_value,
                                    "{option.label}"
                                }
                            }
                        }
                        span {
                            class: "pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-[11px] text-zinc-500",
                            "▼"
                        }
                    }
                    button {
                        r#type: "button",
                        class: "flex h-9 items-center justify-center rounded-lg border border-white/10 bg-zinc-900/70 px-3 text-[12px] font-semibold text-zinc-300 transition hover:border-white/15 hover:bg-zinc-800/80 hover:text-zinc-100",
                        onclick: move |_| {
                            (on_defer.as_ref())(selected_deferral_value.clone());
                            super::super::timer::begin_dismiss_toast(&mut toasts, toast.id);
                        },
                        "Позже"
                    }
                }
            } }

        }
    }
}

fn update_toast_class(exiting: bool) -> &'static str {
    if exiting {
        "toast-item toast-item-exiting pointer-events-auto w-full max-w-[calc(100vw-1.5rem)] overflow-hidden rounded-xl border border-white/[0.08] bg-zinc-950/95 text-zinc-100 shadow-[0_16px_40px_rgba(0,0,0,0.45)] ring-1 ring-black/20 backdrop-blur sm:max-w-none"
    } else {
        "toast-item pointer-events-auto w-full max-w-[calc(100vw-1.5rem)] overflow-hidden rounded-xl border border-white/[0.08] bg-zinc-950/95 text-zinc-100 shadow-[0_16px_40px_rgba(0,0,0,0.45)] ring-1 ring-black/20 backdrop-blur sm:max-w-none"
    }
}

fn update_primary_button_class(disabled: bool) -> &'static str {
    if disabled {
        "flex h-9 cursor-not-allowed items-center justify-center rounded-lg border border-white/[0.06] bg-zinc-900/60 px-3 text-[12px] font-semibold text-zinc-600"
    } else {
        "flex h-9 items-center justify-center rounded-lg bg-blue-500 px-3 text-[12px] font-semibold text-white shadow-[0_6px_18px_rgba(59,130,246,0.16)] transition hover:bg-blue-400 active:translate-y-px"
    }
}

/// Сохраняет выбранную отсрочку в существующем уведомлении.
pub(super) fn set_update_deferral_value(toasts: &mut Signal<Vec<Toast>>, id: u64, value: String) {
    let mut next_toasts = toasts.peek().clone();
    let Some(toast) = next_toasts.iter_mut().find(|toast| toast.id == id) else {
        return;
    };
    let ToastPayload::UpdateAvailable(update) = &mut toast.payload else {
        return;
    };

    update.selected_deferral_value = value;
    toasts.set(next_toasts);
}
