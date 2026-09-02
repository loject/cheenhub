//! Выбор и установка предыдущих стабильных релизов CheenHub.

use dioxus::prelude::*;

use crate::features::application_update::{
    ApplicationUpdateHandle, ApplicationUpdateShutdown, AvailableUpdate, UpdateDownloadStatus,
};
use crate::features::toast::ToastHandle;

use super::{download_status_panel_for_version, download_update_button};

pub(super) fn load_previous_releases(
    handle: ApplicationUpdateHandle,
    mut state: Signal<Option<Result<Vec<AvailableUpdate>, String>>>,
    mut selected_version: Signal<String>,
) {
    state.set(None);

    spawn(async move {
        let result = handle.previous_releases().await;

        if let Ok(releases) = &result {
            if let Some(release) = releases.first() {
                selected_version.set(release.version.clone());
            } else {
                selected_version.set(String::new());
            }
        }

        state.set(Some(result));
    });
}

pub(super) fn previous_releases_panel(
    state: Option<Result<Vec<AvailableUpdate>, String>>,
    releases_state: Signal<Option<Result<Vec<AvailableUpdate>, String>>>,
    mut selected_version: Signal<String>,
    handle: ApplicationUpdateHandle,
    download_status: UpdateDownloadStatus,
    toast: ToastHandle,
    update_shutdown: ApplicationUpdateShutdown,
) -> Element {
    let Some(state) = state else {
        return rsx! {
            div { class: "mt-4 rounded-2xl border border-zinc-800 bg-zinc-950/55 p-4",
                div { class: "flex items-center gap-2",
                    span { class: "inline-block h-3.5 w-3.5 animate-spin rounded-full border-2 border-zinc-600 border-t-zinc-200" }
                    p { class: "text-[12px] font-medium text-zinc-400",
                        "Загружаем предыдущие релизы..."
                    }
                }
            }
        };
    };

    let releases = match state {
        Ok(releases) => releases,
        Err(message) => {
            return rsx! {
                div { class: "mt-4 rounded-2xl border border-red-500/20 bg-red-500/10 p-4",
                    p { class: "text-[13px] font-semibold text-red-100",
                        "Не удалось загрузить предыдущие версии"
                    }
                    p { class: "mt-1 text-[12px] leading-5 text-red-200/75",
                        "{message}"
                    }
                    button {
                        r#type: "button",
                        class: "mt-3 flex h-9 items-center justify-center rounded-xl border border-red-400/25 bg-red-500/10 px-3 text-[12px] font-semibold text-red-100 transition hover:bg-red-500/15 active:scale-[0.96]",
                        onclick: move |_| {
                            load_previous_releases(
                                handle,
                                releases_state,
                                selected_version,
                            );
                        },
                        "Повторить"
                    }
                }
            };
        }
    };

    if releases.is_empty() {
        return rsx! {
            div { class: "mt-4 rounded-2xl border border-zinc-800 bg-zinc-950/55 p-4",
                p { class: "text-[13px] font-semibold text-zinc-200",
                    "Предыдущие версии"
                }
                p { class: "mt-1 text-[12px] leading-5 text-zinc-500",
                    "Для текущей платформы более старых устанавливаемых релизов не найдено."
                }
            }
        };
    }

    let selected_value = selected_version();
    let selected = releases
        .iter()
        .find(|release| release.version == selected_value)
        .cloned()
        .or_else(|| releases.first().cloned());

    let Some(selected) = selected else {
        return rsx! {};
    };

    let selected_title = selected
        .title
        .clone()
        .unwrap_or_else(|| format!("CheenHub {}", selected.version));

    rsx! {
        div { class: "mt-4 rounded-2xl border border-zinc-800 bg-zinc-950/55 p-4",
            div { class: "flex flex-col gap-1",
                h4 { class: "text-[13px] font-semibold text-zinc-100",
                    "Предыдущие версии"
                }
                p { class: "text-[12px] leading-5 text-zinc-500",
                    "Можно установить более ранний стабильный релиз CheenHub."
                }
            }

            div { class: "mt-3 rounded-xl border border-amber-300/15 bg-amber-400/[0.06] px-3 py-2",
                p { class: "text-[11px] leading-5 text-amber-100/80",
                    "Старая версия может не поддерживать данные или настройки, созданные более новой версией приложения."
                }
            }

            label { class: "mt-4 block",
                span { class: "mb-1.5 block text-[11px] font-medium text-zinc-400",
                    "Версия"
                }
                select {
                    class: "h-10 w-full appearance-none rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[12px] font-medium text-zinc-200 outline-none transition focus:border-blue-400/50",
                    value: selected_value,
                    onchange: move |event| {
                        selected_version.set(event.value());
                    },
                    for release in releases.iter() {
                        option {
                            value: release.version.clone(),
                            "v{release.version}"
                        }
                    }
                }
            }

            div { class: "mt-3",
                p { class: "text-[12px] font-medium text-zinc-200",
                    "{selected_title}"
                }
                p { class: "mt-1 text-[11px] text-zinc-500",
                    "Будет установлена версия {selected.version} вместо текущей {handle.current_version()}."
                }
            }

            div { class: "mt-3 flex flex-col gap-2 sm:flex-row sm:items-center",
                a {
                    href: "{selected.release_url}",
                    target: "_blank",
                    rel: "noreferrer",
                    class: "flex h-10 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[12px] font-semibold text-zinc-300 transition hover:border-zinc-700 hover:bg-zinc-900 active:scale-[0.96]",
                    "Открыть релиз"
                }

                {download_update_button(
                    &selected,
                    handle,
                    &download_status,
                    toast,
                    update_shutdown,
                )}
            }

            {download_status_panel_for_version(
                &download_status,
                &selected.version,
            )}
        }
    }
}
