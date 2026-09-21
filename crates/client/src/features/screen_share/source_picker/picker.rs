//! Модальное окно выбора экрана и качества демонстрации.

use dioxus::prelude::*;

use super::model::{
    ScreenShareFrameRate, ScreenShareQualityAdjustment, ScreenShareResolution,
    ScreenShareSelection, ScreenShareSource, ScreenShareSourcePickerState, ScreenShareSourceTab,
};

/// Показывает доступные экраны и возвращает подтверждённый пользователем выбор.
#[component]
pub(crate) fn ScreenShareSourcePicker(
    state: ScreenShareSourcePickerState,
    on_close: EventHandler<()>,
    on_retry: EventHandler<()>,
    on_confirm: EventHandler<ScreenShareSelection>,
) -> Element {
    let sources = match &state {
        ScreenShareSourcePickerState::Ready(sources) => sources.clone(),
        ScreenShareSourcePickerState::Loading | ScreenShareSourcePickerState::Error(_) => {
            Vec::new()
        }
    };
    let error = match &state {
        ScreenShareSourcePickerState::Error(message) => Some(message.clone()),
        ScreenShareSourcePickerState::Loading | ScreenShareSourcePickerState::Ready(_) => None,
    };
    let loading = matches!(state, ScreenShareSourcePickerState::Loading);
    let mut selection = use_signal(|| None::<ScreenShareSelection>);
    let mut active_tab = use_signal(ScreenShareSourceTab::default);
    let mut quality_adjustment = use_signal(|| None::<ScreenShareQualityAdjustment>);
    let effective_selection = ScreenShareSelection::normalized_for(selection().as_ref(), &sources);
    let selected_tab = active_tab();
    let confirmation_summary = if selected_tab == ScreenShareSourceTab::Screens {
        effective_selection
            .as_ref()
            .and_then(|value| value.confirmation_summary(&sources))
    } else {
        None
    };
    let confirm_selection = selected_tab
        .can_confirm(effective_selection.as_ref())
        .then_some(effective_selection.clone())
        .flatten();

    rsx! {
        div {
            class: "fixed inset-0 z-[950] flex items-center justify-center bg-black/70 px-4 py-6 backdrop-blur-sm",
            role: "presentation",
            onclick: move |_| on_close.call(()),
            onkeydown: move |event| {
                if event.key() == Key::Escape {
                    on_close.call(());
                }
            },

            section {
                class: "flex max-h-[min(780px,calc(100vh-48px))] w-full max-w-4xl flex-col overflow-hidden rounded-2xl border border-zinc-800 bg-zinc-950 text-zinc-100 shadow-[0_28px_100px_rgba(0,0,0,0.65)]",
                role: "dialog",
                "aria-modal": "true",
                "aria-labelledby": "screen-share-source-picker-title",
                "aria-describedby": "screen-share-source-picker-description",
                onclick: move |event| event.stop_propagation(),

                header { class: "flex shrink-0 items-start justify-between gap-4 border-b border-zinc-800 px-5 py-4 sm:px-6",
                    div {
                        h2 {
                            id: "screen-share-source-picker-title",
                            class: "text-[17px] font-semibold text-zinc-50",
                            "Демонстрация экрана"
                        }
                        p {
                            id: "screen-share-source-picker-description",
                            class: "mt-1 text-[12px] leading-5 text-zinc-400",
                            "Выберите экран и качество изображения. Звук компьютера передаваться не будет."
                        }
                    }
                    button {
                        r#type: "button",
                        autofocus: true,
                        class: "flex size-9 shrink-0 items-center justify-center rounded-lg text-zinc-400 transition hover:bg-zinc-900 hover:text-zinc-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-400",
                        "aria-label": "Закрыть выбор экрана",
                        onclick: move |_| on_close.call(()),
                        svg { class: "size-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 18 18 6M6 6l12 12" }
                        }
                    }
                }

                div { class: "min-h-0 flex-1 overflow-y-auto px-5 py-5 sm:px-6",
                    div {
                        class: "mb-5 flex w-fit rounded-xl border border-zinc-800 bg-zinc-900/60 p-1",
                        role: "tablist",
                        "aria-label": "Тип источника демонстрации",
                        button {
                            r#type: "button",
                            role: "tab",
                            "aria-selected": if selected_tab == ScreenShareSourceTab::Screens { "true" } else { "false" },
                            class: if selected_tab == ScreenShareSourceTab::Screens {
                                "rounded-lg bg-zinc-700 px-4 py-2 text-[12px] font-semibold text-white shadow-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-400"
                            } else {
                                "rounded-lg px-4 py-2 text-[12px] font-semibold text-zinc-400 transition hover:text-zinc-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-400"
                            },
                            onclick: move |_| {
                                active_tab.set(ScreenShareSourceTab::Screens);
                                debug!(tab = "screens", "screen share source tab changed");
                            },
                            "Экраны"
                        }
                        button {
                            r#type: "button",
                            role: "tab",
                            "aria-selected": if selected_tab == ScreenShareSourceTab::Windows { "true" } else { "false" },
                            class: if selected_tab == ScreenShareSourceTab::Windows {
                                "rounded-lg bg-zinc-700 px-4 py-2 text-[12px] font-semibold text-white shadow-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-400"
                            } else {
                                "rounded-lg px-4 py-2 text-[12px] font-semibold text-zinc-400 transition hover:text-zinc-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-400"
                            },
                            onclick: move |_| {
                                active_tab.set(ScreenShareSourceTab::Windows);
                                debug!(tab = "windows", "screen share source tab changed");
                            },
                            "Окна"
                        }
                    }

                    if selected_tab == ScreenShareSourceTab::Windows {
                        div {
                            class: "flex min-h-64 flex-col items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/35 px-6 py-10 text-center",
                            role: "status",
                            div { class: "flex size-11 items-center justify-center rounded-full bg-zinc-800 text-zinc-300",
                                svg { class: "size-5", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24", "aria-hidden": "true",
                                    rect { x: "3", y: "5", width: "18", height: "14", rx: "2" }
                                    path { stroke_linecap: "round", d: "M3 9h18M7 7h.01M10 7h.01" }
                                }
                            }
                            h3 { class: "mt-3 text-[14px] font-semibold text-zinc-100", "Демонстрация окон появится позже" }
                            p { class: "mt-1 max-w-md text-[12px] leading-5 text-zinc-400", "В ближайших обновлениях здесь можно будет выбрать отдельное окно вместо всего экрана." }
                        }
                    } else if loading {
                        div {
                            class: "space-y-4",
                            role: "status",
                            "aria-live": "polite",
                            "aria-label": "Загружаем доступные экраны",
                            div { class: "grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3",
                                for index in 0..3 {
                                    div { key: "screen-source-skeleton-{index}", class: "overflow-hidden rounded-xl border border-zinc-800 bg-zinc-900/60",
                                        div { class: "aspect-video animate-pulse bg-zinc-800/80" }
                                        div { class: "space-y-2 p-3",
                                            div { class: "h-3 w-2/3 animate-pulse rounded bg-zinc-800" }
                                            div { class: "h-2.5 w-1/3 animate-pulse rounded bg-zinc-800/70" }
                                        }
                                    }
                                }
                            }
                        }
                    } else if let Some(message) = error {
                        div { class: "flex min-h-64 flex-col items-center justify-center rounded-xl border border-red-400/20 bg-red-500/5 px-6 py-10 text-center", role: "alert",
                            div { class: "flex size-11 items-center justify-center rounded-full bg-red-500/10 text-red-300",
                                svg { class: "size-5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 9v4m0 4h.01M10.3 3.9 2.6 17.2A2 2 0 0 0 4.3 20h15.4a2 2 0 0 0 1.7-2.8L13.7 3.9a2 2 0 0 0-3.4 0Z" }
                                }
                            }
                            h3 { class: "mt-3 text-[14px] font-semibold text-zinc-100", "Не удалось показать экраны" }
                            p { class: "mt-1 max-w-md text-[12px] leading-5 text-zinc-400", "{message}" }
                            button {
                                r#type: "button",
                                class: "mt-4 rounded-lg border border-zinc-700 bg-zinc-900 px-4 py-2 text-[12px] font-semibold text-zinc-100 transition hover:border-zinc-600 hover:bg-zinc-800 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-400",
                                onclick: move |_| on_retry.call(()),
                                "Повторить"
                            }
                        }
                    } else if sources.is_empty() {
                        div { class: "flex min-h-64 flex-col items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/35 px-6 py-10 text-center", role: "status",
                            div { class: "flex size-11 items-center justify-center rounded-full bg-zinc-800 text-zinc-300",
                                svg { class: "size-5", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24", "aria-hidden": "true",
                                    rect { x: "3", y: "4", width: "18", height: "12", rx: "2" }
                                    path { stroke_linecap: "round", d: "M8 20h8m-4-4v4" }
                                }
                            }
                            h3 { class: "mt-3 text-[14px] font-semibold text-zinc-100", "Экраны не найдены" }
                            p { class: "mt-1 max-w-md text-[12px] leading-5 text-zinc-400", "Подключите дисплей или обновите список доступных экранов." }
                            button {
                                r#type: "button",
                                class: "mt-4 rounded-lg border border-zinc-700 bg-zinc-900 px-4 py-2 text-[12px] font-semibold text-zinc-100 transition hover:border-zinc-600 hover:bg-zinc-800 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-400",
                                onclick: move |_| on_retry.call(()),
                                "Обновить список"
                            }
                        }
                    } else {
                        div {
                            class: "grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3",
                            role: "radiogroup",
                            "aria-label": "Экран для демонстрации",
                            for source in sources.iter().cloned() {
                                {source_button(source, effective_selection.as_ref(), selection)}
                            }
                        }

                        fieldset { class: "mt-6 border-t border-zinc-800 pt-5",
                            legend { class: "px-1 text-[13px] font-semibold text-zinc-200", "Качество" }
                            div { class: "mt-3 grid gap-3 sm:grid-cols-2",
                                label { class: "block",
                                    span { class: "mb-1.5 block text-[11px] font-medium text-zinc-400", "Разрешение" }
                                    select {
                                        class: "h-11 w-full rounded-xl border border-zinc-700 bg-zinc-900 px-3 text-[13px] font-semibold text-zinc-100 outline-none transition hover:border-zinc-600 focus:border-blue-400 focus:ring-2 focus:ring-blue-400/25",
                                        value: effective_selection.as_ref().map_or(ScreenShareResolution::P720.value(), |value| value.resolution.value()),
                                        onchange: {
                                            let current = effective_selection.clone();
                                            move |event: Event<FormData>| {
                                                let Some(resolution) = ScreenShareResolution::from_value(&event.value()) else {
                                                    warn!(value = %event.value(), "unknown screen share resolution selected");
                                                    return;
                                                };
                                                if let Some(mut next) = current.clone() {
                                                    let adjustment = next.select_resolution(resolution);
                                                    info!(resolution = ?next.resolution, frame_rate = ?next.frame_rate, adjusted = adjustment.is_some(), "screen share quality changed");
                                                    selection.set(Some(next));
                                                    quality_adjustment.set(adjustment);
                                                }
                                            }
                                        },
                                        option { value: "720", "720p" }
                                        option { value: "1080", "1080p · до 15 FPS" }
                                    }
                                }
                                label { class: "block",
                                    span { class: "mb-1.5 block text-[11px] font-medium text-zinc-400", "Частота кадров" }
                                    select {
                                        class: "h-11 w-full rounded-xl border border-zinc-700 bg-zinc-900 px-3 text-[13px] font-semibold text-zinc-100 outline-none transition hover:border-zinc-600 focus:border-blue-400 focus:ring-2 focus:ring-blue-400/25",
                                        value: effective_selection.as_ref().map_or(ScreenShareFrameRate::Fps30.value(), |value| value.frame_rate.value()),
                                        onchange: {
                                            let current = effective_selection.clone();
                                            move |event: Event<FormData>| {
                                                let Some(frame_rate) = ScreenShareFrameRate::from_value(&event.value()) else {
                                                    warn!(value = %event.value(), "unknown screen share frame rate selected");
                                                    return;
                                                };
                                                if let Some(mut next) = current.clone() {
                                                    let adjustment = next.select_frame_rate(frame_rate);
                                                    info!(resolution = ?next.resolution, frame_rate = ?next.frame_rate, adjusted = adjustment.is_some(), "screen share quality changed");
                                                    selection.set(Some(next));
                                                    quality_adjustment.set(adjustment);
                                                }
                                            }
                                        },
                                        option { value: "15", "15 FPS" }
                                        option { value: "30", "30 FPS · до 720p" }
                                    }
                                }
                            }
                            p { class: "mt-2 text-[11px] leading-5 text-zinc-500", "1080p работает до 15 FPS, а 30 FPS — до 720p. При выборе мы автоматически настроим второй параметр." }
                            if let Some(adjustment) = quality_adjustment() {
                                div {
                                    class: "mt-3 flex items-start gap-2 rounded-lg border border-blue-400/20 bg-blue-500/10 px-3 py-2.5 text-[11px] leading-5 text-blue-100",
                                    role: "status",
                                    "aria-live": "polite",
                                    svg { class: "mt-0.5 size-3.5 shrink-0", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 16v-4m0-4h.01M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z" }
                                    }
                                    span { "{adjustment.message()}" }
                                }
                            }
                        }
                    }
                }

                footer { class: "flex shrink-0 flex-col gap-3 border-t border-zinc-800 bg-zinc-950/95 px-5 py-4 sm:flex-row sm:items-center sm:justify-between sm:px-6",
                    div { class: "min-w-0",
                        p { class: "flex items-center gap-1.5 text-[12px] font-medium text-amber-200",
                            svg { class: "size-3.5 shrink-0", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M11 5 6 9H3v6h3l5 4V5Zm5.5 5.5 4 4m0-4-4 4" }
                            }
                            "Без звука"
                        }
                        if let Some(summary) = confirmation_summary {
                            p { class: "mt-1 truncate text-[11px] text-zinc-500", "{summary}" }
                        }
                    }
                    div { class: "flex shrink-0 items-center justify-end gap-2",
                        button {
                            r#type: "button",
                            class: "h-10 rounded-lg px-4 text-[13px] font-semibold text-zinc-300 transition hover:bg-zinc-900 hover:text-zinc-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-400",
                            onclick: move |_| on_close.call(()),
                            "Отмена"
                        }
                        button {
                            r#type: "button",
                            disabled: confirm_selection.is_none(),
                            title: if selected_tab == ScreenShareSourceTab::Windows { "Демонстрация отдельных окон пока недоступна" } else { "" },
                            class: "h-10 rounded-lg bg-blue-500 px-5 text-[13px] font-semibold text-white transition hover:bg-blue-400 disabled:cursor-not-allowed disabled:bg-zinc-800 disabled:text-zinc-500 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-300",
                            onclick: move |_| {
                                if let Some(value) = confirm_selection.clone() {
                                    info!(source_id = %value.source_id, resolution = ?value.resolution, frame_rate = ?value.frame_rate, "screen share source selection confirmed");
                                    on_confirm.call(value);
                                }
                            },
                            "Начать демонстрацию"
                        }
                    }
                }
            }
        }
    }
}

fn source_button(
    source: ScreenShareSource,
    current: Option<&ScreenShareSelection>,
    mut selection: Signal<Option<ScreenShareSelection>>,
) -> Element {
    let selected = current.is_some_and(|value| value.source_id == source.id);
    let resolution = current.map_or_else(ScreenShareResolution::default, |value| value.resolution);
    let frame_rate = current.map_or_else(ScreenShareFrameRate::default, |value| value.frame_rate);
    let source_id = source.id.clone();
    let source_label = if source.is_primary {
        format!(
            "{}, основной, {} на {} пикселей",
            source.name, source.width, source.height
        )
    } else {
        format!(
            "{}, {} на {} пикселей",
            source.name, source.width, source.height
        )
    };

    rsx! {
        button {
            key: "{source.id}",
            r#type: "button",
            role: "radio",
            "aria-checked": if selected { "true" } else { "false" },
            "aria-label": "{source_label}",
            class: if selected {
                "group overflow-hidden rounded-xl border border-blue-400/70 bg-blue-500/10 text-left shadow-[0_0_0_2px_rgba(59,130,246,0.12)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-300"
            } else {
                "group overflow-hidden rounded-xl border border-zinc-800 bg-zinc-900/45 text-left transition hover:border-zinc-700 hover:bg-zinc-900 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-400"
            },
            onclick: move |_| selection.set(Some(ScreenShareSelection {
                source_id: source_id.clone(),
                resolution,
                frame_rate,
            })),
            div { class: "relative aspect-video overflow-hidden bg-zinc-900",
                img {
                    class: "h-full w-full object-contain transition duration-200 group-hover:scale-[1.01]",
                    src: "{source.preview_url}",
                    alt: "Предварительный просмотр: {source.name}",
                }
                if source.is_primary {
                    span { class: "absolute left-2 top-2 rounded-md border border-white/10 bg-black/70 px-2 py-1 text-[10px] font-semibold text-white backdrop-blur-sm", "Основной" }
                }
                if selected {
                    span { class: "absolute right-2 top-2 flex size-6 items-center justify-center rounded-full bg-blue-500 text-white shadow-lg", "aria-hidden": "true",
                        svg { class: "size-3.5", fill: "none", stroke: "currentColor", stroke_width: "2.4", view_box: "0 0 24 24",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "m5 12 4 4L19 6" }
                        }
                    }
                }
            }
            div { class: "flex items-center justify-between gap-3 px-3 py-2.5",
                span { class: "truncate text-[12px] font-semibold text-zinc-100", "{source.name}" }
                span { class: "shrink-0 text-[10px] text-zinc-500", "{source.width}×{source.height}" }
            }
        }
    }
}
