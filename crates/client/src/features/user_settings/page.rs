//! User settings modal shell.

use dioxus::prelude::*;

use super::logout_section::LogoutSettingsSection;
use super::profile_section::ProfileSettingsSection;
use super::security_section::SecuritySettingsSection;
use super::sound_section::SoundSettingsSection;

/// User settings sections shown in the modal menu.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum UserSettingsSection {
    /// Profile and account settings.
    Profile,
    /// Audio input and output settings.
    Sound,
    /// Account security and active sessions.
    Security,
    /// Sign-out action section.
    Logout,
}

#[derive(Clone, Copy)]
struct UserSettingsSectionMeta {
    kind: UserSettingsSection,
    label: &'static str,
    description: &'static str,
}

const SETTINGS_SECTIONS: &[UserSettingsSectionMeta] = &[
    UserSettingsSectionMeta {
        kind: UserSettingsSection::Profile,
        label: "Профиль",
        description: "Аватар и учетные данные",
    },
    UserSettingsSectionMeta {
        kind: UserSettingsSection::Sound,
        label: "Звук",
        description: "Ввод, вывод и активация",
    },
    UserSettingsSectionMeta {
        kind: UserSettingsSection::Security,
        label: "Безопасность",
        description: "Устройства и сеансы",
    },
    UserSettingsSectionMeta {
        kind: UserSettingsSection::Logout,
        label: "Выйти",
        description: "Завершение сеанса",
    },
];

/// Renders a compact nearly-fullscreen user settings modal.
#[component]
pub(crate) fn UserSettingsPage(
    active_section: UserSettingsSection,
    on_select_section: EventHandler<UserSettingsSection>,
    on_close: EventHandler<()>,
) -> Element {
    let section_label = section_label(active_section);

    rsx! {
        div { class: "fixed inset-0 z-[100] flex items-stretch justify-center bg-black/70 p-0 backdrop-blur-sm md:items-center md:p-3",
            button {
                r#type: "button",
                class: "absolute inset-0 cursor-default",
                "aria-label": "Закрыть настройки пользователя",
                onclick: move |_| on_close.call(()),
            }
            section {
                role: "dialog",
                "aria-modal": "true",
                "aria-label": "Настройки пользователя",
                class: "relative flex h-[100dvh] w-full max-w-none flex-col overflow-hidden rounded-none border-0 bg-zinc-950 text-zinc-100 shadow-none md:h-[calc(100vh-140px)] md:max-w-[1240px] md:flex-row md:rounded-2xl md:border md:border-zinc-800 md:shadow-[0_30px_110px_rgba(0,0,0,.65)]",
                nav { class: "order-2 flex shrink-0 flex-col border-b border-zinc-800/80 bg-zinc-950/95 p-2.5 md:order-none md:w-[272px] md:border-b-0 md:border-r md:bg-zinc-950/80 md:p-3",
                    div { class: "hidden px-1 md:mb-4 md:block",
                        p { class: "text-[10px] font-medium uppercase tracking-[0.22em] text-zinc-600", "Настройки" }
                        h1 { class: "mt-1.5 text-[16px] font-semibold tracking-[-0.03em] text-zinc-50", "Пользователь" }
                    }
                    div { class: "flex gap-2 overflow-x-auto pb-1 md:block md:space-y-1 md:overflow-visible md:pb-0",
                        for section in SETTINGS_SECTIONS {
                            button {
                                key: "{section.label}",
                                r#type: "button",
                                class: settings_item_class(active_section == section.kind, section.kind),
                                "aria-current": if active_section == section.kind { "page" } else { "false" },
                                onclick: move |_| on_select_section.call(section.kind),
                                span { class: "block whitespace-nowrap text-[12px] font-medium md:whitespace-normal", "{section.label}" }
                                span { class: "mt-0.5 hidden text-[11px] leading-4 text-zinc-500 md:block", "{section.description}" }
                            }
                        }
                    }
                }
                div { class: "contents md:block md:min-w-0 md:flex-1 md:overflow-y-auto md:bg-zinc-950/35",
                    div { class: "order-1 z-10 flex h-14 shrink-0 items-center justify-between gap-3 border-b border-zinc-800/80 bg-zinc-950/90 px-4 backdrop-blur-xl md:sticky md:top-0 md:h-16 md:gap-4 md:bg-zinc-950/85 md:px-5",
                        div { class: "min-w-0",
                            p { class: "text-[10px] font-medium uppercase tracking-[0.18em] text-zinc-600", "Общие настройки" }
                            h2 { class: "truncate text-[14px] font-semibold tracking-[-0.03em] text-zinc-50", "{section_label}" }
                        }
                        button {
                            r#type: "button",
                            class: "group relative flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 text-zinc-400 transition hover:border-zinc-700 hover:bg-zinc-900 hover:text-zinc-100 md:h-9 md:w-9",
                            "aria-label": "Закрыть настройки пользователя",
                            onclick: move |_| on_close.call(()),
                            span { class: "pointer-events-none absolute right-0 top-[calc(100%+8px)] hidden -translate-y-1 whitespace-nowrap rounded-xl border border-zinc-800 bg-zinc-950/95 px-3 py-1.5 text-[12px] font-medium text-zinc-200 opacity-0 shadow-[0_16px_40px_rgba(0,0,0,.45)] backdrop-blur-xl transition-[opacity,transform] duration-150 md:block group-hover:translate-y-0 group-hover:opacity-100 group-focus-visible:translate-y-0 group-focus-visible:opacity-100", "Закрыть" }
                            svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 18 18 6M6 6l12 12" }
                            }
                        }
                    }
                    div { class: "order-3 min-h-0 w-full flex-1 overflow-y-auto bg-zinc-950/35 px-4 py-4 pb-6 md:mx-auto md:max-w-[980px] md:overflow-visible md:bg-transparent md:px-6 md:py-6",
                        match active_section {
                            UserSettingsSection::Profile => rsx! {
                                ProfileSettingsSection {}
                            },
                            UserSettingsSection::Sound => rsx! {
                                SoundSettingsSection {}
                            },
                            UserSettingsSection::Security => rsx! {
                                SecuritySettingsSection {}
                            },
                            UserSettingsSection::Logout => rsx! {
                                LogoutSettingsSection {}
                            },
                        }
                    }
                }
            }
        }
    }
}

fn settings_item_class(active: bool, section: UserSettingsSection) -> &'static str {
    if active && section == UserSettingsSection::Logout {
        "flex min-h-[44px] min-w-[112px] shrink-0 flex-col items-center justify-center rounded-xl border border-red-500/25 bg-red-500/10 px-3 py-2.5 text-center text-red-100 md:min-h-0 md:w-full md:min-w-0 md:shrink md:items-stretch md:justify-start md:py-2 md:text-left"
    } else if active {
        "flex min-h-[44px] min-w-[112px] shrink-0 flex-col items-center justify-center rounded-xl border border-accent/25 bg-accent/10 px-3 py-2.5 text-center text-blue-100 md:min-h-0 md:w-full md:min-w-0 md:shrink md:items-stretch md:justify-start md:py-2 md:text-left"
    } else if section == UserSettingsSection::Logout {
        "flex min-h-[44px] min-w-[112px] shrink-0 flex-col items-center justify-center rounded-xl border border-transparent px-3 py-2.5 text-center text-red-300 transition hover:border-red-500/20 hover:bg-red-500/10 hover:text-red-200 md:min-h-0 md:w-full md:min-w-0 md:shrink md:items-stretch md:justify-start md:py-2 md:text-left"
    } else {
        "flex min-h-[44px] min-w-[112px] shrink-0 flex-col items-center justify-center rounded-xl border border-transparent px-3 py-2.5 text-center text-zinc-300 transition hover:border-zinc-800 hover:bg-zinc-900 hover:text-zinc-100 md:min-h-0 md:w-full md:min-w-0 md:shrink md:items-stretch md:justify-start md:py-2 md:text-left"
    }
}

fn section_label(section: UserSettingsSection) -> &'static str {
    match section {
        UserSettingsSection::Profile => "Профиль",
        UserSettingsSection::Sound => "Звук",
        UserSettingsSection::Security => "Безопасность",
        UserSettingsSection::Logout => "Выйти",
    }
}
