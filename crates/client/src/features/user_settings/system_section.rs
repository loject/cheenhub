//! Раздел системных настроек пользователя.

use dioxus::prelude::*;

use crate::features::autostart::AutostartHandle;
use crate::features::system_tray::SystemTrayHandle;

use super::update_section::UpdateSettingsSection;

/// Рендерит системные настройки клиента.
#[component]
pub(crate) fn SystemSettingsSection() -> Element {
    let system_tray = use_context::<SystemTrayHandle>();
    let autostart = use_context::<AutostartHandle>();
    let minimize_to_tray_on_close = system_tray.minimize_to_tray_on_close();
    let toggle_system_tray = system_tray.clone();
    let autostart_enabled = autostart.enabled();
    let toggle_autostart = autostart.clone();

    rsx! {
        div { class: "space-y-4",
            UpdateSettingsSection {}
            div { class: "rounded-2xl border border-zinc-800 bg-zinc-950/70 p-4",
            div {
                h3 { class: "text-[16px] font-semibold tracking-[-0.03em] text-zinc-50", "Система" }
                p { class: "mt-1 text-[12px] leading-5 text-zinc-500", "Поведение окна CheenHub и системного трея." }
            }

            div { class: "mt-4 rounded-2xl border border-zinc-800 bg-zinc-900/45 p-4",
                label { class: "flex cursor-pointer items-center justify-between gap-4",
                    span { class: "min-w-0",
                        span { class: "block text-[14px] font-medium text-zinc-100", "Сворачивать в трей при закрытии" }
                        span { class: "mt-1 block text-[12px] leading-5 text-zinc-500", "В desktop-приложении кнопка закрытия будет скрывать окно, а CheenHub продолжит работать в системном трее." }
                    }
                    input {
                        r#type: "checkbox",
                        class: "peer sr-only",
                        checked: minimize_to_tray_on_close,
                        onchange: move |event| {
                            toggle_system_tray.set_minimize_to_tray_on_close(event.checked());
                        },
                    }
                    span {
                        "aria-hidden": "true",
                        class: toggle_class(minimize_to_tray_on_close),
                        span { class: knob_class(minimize_to_tray_on_close) }
                    }
                }
            }
            if autostart.is_supported() {
                div { class: "mt-3 rounded-2xl border border-zinc-800 bg-zinc-900/45 p-4",
                    label { class: "flex cursor-pointer items-center justify-between gap-4",
                        span { class: "min-w-0",
                            span { class: "block text-[14px] font-medium text-zinc-100", "Запускать CheenHub вместе с Windows" }
                            span { class: "mt-1 block text-[12px] leading-5 text-zinc-500", "После входа в Windows CheenHub запустится в системном трее, не открывая главное окно." }
                        }
                        input {
                            r#type: "checkbox",
                            class: "peer sr-only",
                            checked: autostart_enabled,
                            onchange: move |event| {
                                toggle_autostart.set_enabled(event.checked());
                            },
                        }
                        span {
                            "aria-hidden": "true",
                            class: toggle_class(autostart_enabled),
                            span { class: knob_class(autostart_enabled) }
                        }
                    }
                    if let Some(error) = autostart.error() {
                        p { class: "mt-3 text-[12px] leading-5 text-red-300", "{error}" }
                    }
                }
            }
            }
        }
    }
}

fn toggle_class(enabled: bool) -> &'static str {
    if enabled {
        "relative inline-flex h-6 w-11 shrink-0 items-center rounded-full bg-blue-500 transition"
    } else {
        "relative inline-flex h-6 w-11 shrink-0 items-center rounded-full bg-zinc-700 transition"
    }
}

fn knob_class(enabled: bool) -> &'static str {
    if enabled {
        "absolute left-6 h-4 w-4 rounded-full bg-white shadow-sm transition"
    } else {
        "absolute left-1 h-4 w-4 rounded-full bg-white shadow-sm transition"
    }
}
