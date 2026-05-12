//! Mock user settings modal shell.

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
        div { class: "fixed inset-0 z-[100] flex items-center justify-center bg-black/70 p-3 backdrop-blur-sm",
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
                class: "relative flex h-[calc(100vh-24px)] w-full max-w-[1240px] overflow-hidden rounded-2xl border border-zinc-800 bg-zinc-950 text-zinc-100 shadow-[0_30px_110px_rgba(0,0,0,.65)] md:h-[calc(100vh-140px)]",
                nav { class: "flex w-[272px] shrink-0 flex-col border-r border-zinc-800/80 bg-zinc-950/80 p-3",
                    div { class: "mb-4 px-1",
                        p { class: "text-[10px] font-medium uppercase tracking-[0.22em] text-zinc-600", "Настройки" }
                        h1 { class: "mt-1.5 text-[16px] font-semibold tracking-[-0.03em] text-zinc-50", "Пользователь" }
                    }
                    div { class: "space-y-1",
                        for section in SETTINGS_SECTIONS {
                            button {
                                key: "{section.label}",
                                r#type: "button",
                                class: settings_item_class(active_section == section.kind, section.kind),
                                "aria-current": if active_section == section.kind { "page" } else { "false" },
                                onclick: move |_| on_select_section.call(section.kind),
                                span { class: "block text-[12px] font-medium", "{section.label}" }
                                span { class: "mt-0.5 block text-[11px] leading-4 text-zinc-500", "{section.description}" }
                            }
                        }
                    }
                }
                div { class: "min-w-0 flex-1 overflow-y-auto bg-zinc-950/35",
                    div { class: "sticky top-0 z-10 flex h-16 items-center justify-between gap-4 border-b border-zinc-800/80 bg-zinc-950/85 px-5 backdrop-blur-xl",
                        div { class: "min-w-0",
                            p { class: "text-[10px] font-medium uppercase tracking-[0.18em] text-zinc-600", "Общие настройки" }
                            h2 { class: "truncate text-[14px] font-semibold tracking-[-0.03em] text-zinc-50", "{section_label}" }
                        }
                        button {
                            r#type: "button",
                            class: "group relative flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 text-zinc-400 transition hover:border-zinc-700 hover:bg-zinc-900 hover:text-zinc-100",
                            "aria-label": "Закрыть настройки пользователя",
                            onclick: move |_| on_close.call(()),
                            span { class: "pointer-events-none absolute right-0 top-[calc(100%+8px)] -translate-y-1 whitespace-nowrap rounded-xl border border-zinc-800 bg-zinc-950/95 px-3 py-1.5 text-[12px] font-medium text-zinc-200 opacity-0 shadow-[0_16px_40px_rgba(0,0,0,.45)] backdrop-blur-xl transition-[opacity,transform] duration-150 group-hover:translate-y-0 group-hover:opacity-100 group-focus-visible:translate-y-0 group-focus-visible:opacity-100", "Закрыть" }
                            svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 18 18 6M6 6l12 12" }
                            }
                        }
                    }
                    div { class: "mx-auto w-full max-w-[980px] px-6 py-6",
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
                                LogoutSettingsSection { on_select_section }
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
        "flex w-full flex-col rounded-xl border border-red-500/25 bg-red-500/10 px-3 py-2 text-left text-red-100"
    } else if active {
        "flex w-full flex-col rounded-xl border border-accent/25 bg-accent/10 px-3 py-2 text-left text-blue-100"
    } else if section == UserSettingsSection::Logout {
        "flex w-full flex-col rounded-xl border border-transparent px-3 py-2 text-left text-red-300 transition hover:border-red-500/20 hover:bg-red-500/10 hover:text-red-200"
    } else {
        "flex w-full flex-col rounded-xl border border-transparent px-3 py-2 text-left text-zinc-300 transition hover:border-zinc-800 hover:bg-zinc-900 hover:text-zinc-100"
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
