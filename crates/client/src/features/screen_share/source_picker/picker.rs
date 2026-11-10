//! Модальное окно выбора экрана и качества демонстрации.
//!
//! Окно показывает доступные экраны и передаёт выбранное качество при запуске.

mod assessment_card;
mod bitrate_slider;
mod browser_intro;
mod decor;
mod empty_state;
mod kind_tabs;
mod list_skeleton;
mod preset_grid;
mod quality_panel;
mod segmented;
mod source_card;

use dioxus::prelude::*;

use self::browser_intro::BrowserIntro;
use self::decor::aria_flag;
use self::empty_state::{EmptyState, EmptyStateTone};
use self::kind_tabs::KindTabs;
use self::list_skeleton::ListSkeleton;
use self::quality_panel::{QualityChange, QualityPanel};
use self::source_card::SourceCard;

use super::model::{ScreenShareSelection, ScreenShareSourcePickerState, ScreenShareSourceTab};

/// Показывает доступные экраны и возвращает подтверждённый пользователем выбор.
#[component]
pub(crate) fn ScreenShareSourcePicker(
    state: ScreenShareSourcePickerState,
    initial_selection: Option<ScreenShareSelection>,
    on_close: EventHandler<()>,
    on_retry: EventHandler<()>,
    on_confirm: EventHandler<ScreenShareSelection>,
) -> Element {
    let browser_mode = matches!(state, ScreenShareSourcePickerState::Browser);
    let sources = match &state {
        ScreenShareSourcePickerState::Ready(sources) => sources.clone(),
        ScreenShareSourcePickerState::Browser
        | ScreenShareSourcePickerState::Loading
        | ScreenShareSourcePickerState::Error(_) => Vec::new(),
    };
    let error = match &state {
        ScreenShareSourcePickerState::Error(message) => Some(message.clone()),
        ScreenShareSourcePickerState::Browser
        | ScreenShareSourcePickerState::Loading
        | ScreenShareSourcePickerState::Ready(_) => None,
    };
    let loading = matches!(state, ScreenShareSourcePickerState::Loading);

    let mut selection = use_signal(|| initial_selection);
    let mut active_tab = use_signal(ScreenShareSourceTab::default);

    let effective_selection = if browser_mode {
        Some(
            selection()
                .filter(|value| value.source_id.is_none())
                .unwrap_or_else(ScreenShareSelection::browser_default),
        )
    } else {
        ScreenShareSelection::normalized_for(selection().as_ref(), &sources)
    };
    let resolution = effective_selection
        .as_ref()
        .map_or_else(Default::default, |value| value.resolution);
    let frame_rate = effective_selection
        .as_ref()
        .map_or_else(Default::default, |value| value.frame_rate);
    let selected_tab = active_tab();
    let bitrate_selection = effective_selection.clone();
    let selected_source = effective_selection.as_ref().and_then(|value| {
        value
            .source_id
            .as_deref()
            .and_then(|id| sources.iter().find(|source| source.id == id).cloned())
    });
    let quality_summary = format!("{} · {}", resolution.title(), frame_rate.title());
    let confirm_selection = selected_tab
        .can_confirm(effective_selection.as_ref())
        .then(|| effective_selection.clone())
        .flatten();
    let start_label = if browser_mode {
        "Выбрать экран и начать"
    } else {
        "Начать демонстрацию"
    };
    let source_count_label = match sources.len() {
        0 => "Нет доступных".to_owned(),
        1 => "1 экран".to_owned(),
        count => format!("{count} экрана"),
    };

    rsx! {
        div {
            class: "screen-share-backdrop fixed inset-0 z-[950] flex items-center justify-center bg-black/70 px-3 py-4 backdrop-blur-sm",
            role: "presentation",
            onclick: move |_| on_close.call(()),
            onkeydown: move |event| {
                if event.key() == Key::Named(NamedKey::Escape) {
                    on_close.call(());
                }
            },

            section {
                class: "screen-share-modal flex max-h-[calc(100vh_-_32px)] w-full max-w-[920px] flex-col overflow-hidden rounded-xl border border-zinc-800 bg-zinc-950 text-zinc-100",
                role: "dialog",
                "aria-modal": "true",
                "aria-labelledby": "screen-share-source-picker-title",
                "aria-describedby": "screen-share-source-picker-description",
                onclick: move |event| event.stop_propagation(),

                header { class: "flex shrink-0 items-center gap-3 border-b border-zinc-800 px-4 py-3",
                    div { class: "grid h-9 w-9 shrink-0 place-items-center rounded-lg bg-blue-500/15 text-blue-300",
                        svg { class: "h-[18px] w-[18px]", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24", "aria-hidden": "true",
                            rect { x: "3", y: "4", width: "18", height: "12", rx: "2" }
                            path { stroke_linecap: "round", d: "M8 20h8m-4-4v4M12 12v-4m-2 2 2-2 2 2" }
                        }
                    }
                    div { class: "min-w-0 flex-1",
                        h2 {
                            id: "screen-share-source-picker-title",
                            class: "truncate text-[15px] font-semibold tracking-tight text-white",
                            "Демонстрация экрана"
                        }
                        p {
                            id: "screen-share-source-picker-description",
                            class: "mt-0.5 truncate text-[11.5px] leading-4 text-zinc-400",
                            if browser_mode {
                                "Настройте качество — источник выберете в системном окне."
                            } else {
                                "Выберите экран и качество изображения."
                            }
                        }
                    }
                    button {
                        r#type: "button",
                        autofocus: true,
                        class: "grid h-8 w-8 shrink-0 place-items-center rounded-md text-zinc-400 transition-[background-color,color] duration-200 hover:bg-white/[.06] hover:text-white active:scale-[0.96] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-300/70",
                        "aria-label": "Закрыть выбор экрана",
                        onclick: move |_| on_close.call(()),
                        svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 18 18 6M6 6l12 12" }
                        }
                    }
                }

                div { class: "min-h-0 flex-1 overflow-y-auto overscroll-contain",
                    div { class: "screen-share-fade-up grid items-start gap-5 p-4 lg:grid-cols-[minmax(0,1.2fr)_minmax(0,1fr)] lg:gap-6",
                        div { class: "min-w-0 space-y-3",
                            if !browser_mode {
                                div {
                                    h3 { class: "mb-2 text-[13px] font-semibold tracking-tight text-white", "Что показать" }
                                    KindTabs {
                                        active: selected_tab,
                                        on_change: move |tab| {
                                            active_tab.set(tab);
                                            debug!(tab = ?tab, "screen share source tab changed");
                                        },
                                    }
                                }

                                div { class: "min-h-[248px]",
                                div { key: "{selected_tab:?}", class: "screen-share-fade-in",
                                if selected_tab == ScreenShareSourceTab::Windows {
                                    EmptyState {
                                        title: "Нет доступных окон",
                                        body: "Откройте нужное приложение или вернитесь на вкладку «Весь экран», чтобы показать экран целиком.",
                                    }
                                } else if loading {
                                    ListSkeleton {}
                                } else if let Some(message) = error.clone() {
                                    EmptyState {
                                        title: "Не удалось показать экраны",
                                        body: message,
                                        action_label: "Повторить",
                                        tone: EmptyStateTone::Danger,
                                        on_action: move |_| on_retry.call(()),
                                    }
                                } else if sources.is_empty() {
                                    EmptyState {
                                        title: "Экраны не найдены",
                                        body: "Подключите дисплей или обновите список доступных экранов.",
                                        action_label: "Обновить список",
                                        on_action: move |_| on_retry.call(()),
                                    }
                                } else {
                                    div {
                                        class: "space-y-2",
                                        div { class: "flex items-baseline justify-between gap-2",
                                            p { class: "text-[11px] font-medium text-zinc-400", "Доступные экраны" }
                                            p { class: "text-[10.5px] tabular-nums text-zinc-500", "{source_count_label}" }
                                        }
                                        div {
                                            class: "grid grid-cols-2 gap-2",
                                            role: "radiogroup",
                                            "aria-label": "Экран для демонстрации",
                                            for source in sources.iter().cloned() {
                                                {
                                                    let current_selection = effective_selection.clone();
                                                    rsx! {
                                                        SourceCard {
                                                            key: "{source.id}",
                                                            selected: current_selection.as_ref().is_some_and(|value| {
                                                                value.source_id.as_deref() == Some(source.id.as_str())
                                                            }),
                                                            source,
                                                            on_select: move |source_id: String| {
                                                                let mut next = current_selection.clone().unwrap_or_else(|| {
                                                                    ScreenShareSelection::browser_default()
                                                                });
                                                                next.source_id = Some(source_id);
                                                                selection.set(Some(next));
                                                            },
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                }
                                }
                            } else {
                                BrowserIntro {}
                            }
                        }

                        div { class: "min-w-0",
                            QualityPanel {
                                resolution,
                                frame_rate,
                                bitrate_bps: effective_selection.as_ref().map_or(cheenhub_contracts::video_presets::SCREEN_SHARE_BITRATE_LIMIT.max_bitrate_bps, |value| value.bitrate_bps),
                                on_bitrate_change: move |bitrate_bps: u32| {
                                    let mut next = bitrate_selection.clone().unwrap_or_else(ScreenShareSelection::browser_default);
                                    next.bitrate_bps = bitrate_bps;
                                    selection.set(Some(next));
                                },
                                on_change: move |change: QualityChange| {
                                    debug!(
                                        resolution = ?change.resolution,
                                        frame_rate = ?change.frame_rate,
                                        "screen share quality changed"
                                    );
                                    let mut next = effective_selection
                                        .clone()
                                        .unwrap_or_else(ScreenShareSelection::browser_default);
                                    next.resolution = change.resolution;
                                    next.frame_rate = change.frame_rate;
                                    selection.set(Some(next));
                                },
                            }
                        }
                    }
                }

                footer { class: "flex shrink-0 flex-col gap-2.5 border-t border-zinc-800 bg-zinc-950/95 px-4 py-3 sm:flex-row sm:items-center sm:justify-between",
                    div { class: "flex min-w-0 items-center gap-2.5",
                        div { class: "grid h-8 w-8 shrink-0 place-items-center rounded-md bg-white/[.06] text-zinc-300",
                            svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24", "aria-hidden": "true",
                                rect { x: "3", y: "4", width: "18", height: "12", rx: "2" }
                                path { stroke_linecap: "round", d: "M8 20h8m-4-4v4" }
                            }
                        }
                        div { class: "min-w-0",
                            p { class: "text-[10px] font-medium uppercase tracking-wider text-zinc-500",
                                if browser_mode { "Что показывать" } else { "Будет показано" }
                            }
                            p { class: "truncate text-[12px] font-medium text-zinc-100",
                                if browser_mode {
                                    "Источник выбирается в системном окне"
                                    span { class: "font-normal text-zinc-400", " · {quality_summary}" }
                                } else if let Some(source) = selected_source.as_ref() {
                                    "{source.name}"
                                    span { class: "font-normal text-zinc-400", " · {quality_summary}" }
                                } else {
                                    "Экран не выбран"
                                }
                            }
                        }
                    }
                    div { class: "flex shrink-0 items-center justify-end gap-2",
                        button {
                            r#type: "button",
                            class: "h-9 rounded-lg px-3 text-[12.5px] font-medium text-zinc-300 transition-[background-color,color] duration-200 hover:bg-white/[.06] hover:text-white active:scale-[0.96] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-white/30",
                            onclick: move |_| on_close.call(()),
                            "Отмена"
                        }
                        button {
                            r#type: "button",
                            disabled: confirm_selection.is_none(),
                            "aria-disabled": aria_flag(confirm_selection.is_none()),
                            class: "inline-flex h-9 items-center justify-center gap-2 rounded-lg bg-blue-500 px-4 text-[12.5px] font-semibold text-white transition-[background-color,opacity] duration-200 hover:bg-blue-400 active:scale-[0.96] disabled:cursor-not-allowed disabled:bg-zinc-800 disabled:text-zinc-500 disabled:active:scale-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-300",
                            onclick: move |_| {
                                if let Some(value) = confirm_selection.clone() {
                                    info!(source_id = ?value.source_id, resolution = ?value.resolution, frame_rate = ?value.frame_rate, bitrate_bps = value.bitrate_bps, "screen share source selection confirmed");
                                    on_confirm.call(value);
                                }
                            },
                            "{start_label}"
                        }
                    }
                }
            }
        }
    }
}
