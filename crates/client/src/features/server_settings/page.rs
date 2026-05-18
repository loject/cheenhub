//! Server settings page.

use cheenhub_contracts::rest::ServerSummary;
use dioxus::prelude::*;

use super::invites_section::ServerInvitesSettingsSection;
use super::members_section::ServerMembersSettingsSection;
use super::overview_section::ServerOverviewSettingsSection;
use super::roles_section::ServerRolesSettingsSection;

/// Server settings sections shown in the settings menu.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum ServerSettingsSection {
    /// Server overview section.
    Overview,
    /// Server invite-link management section.
    Invites,
    /// Member management section.
    Members,
    /// Role management section.
    Roles,
    /// Safety and moderation section.
    Moderation,
    /// Voice behavior section.
    Voice,
}

#[derive(Clone, Copy)]
struct SettingsSectionMeta {
    kind: ServerSettingsSection,
    label: &'static str,
    description: &'static str,
}

const SETTINGS_SECTIONS: &[SettingsSectionMeta] = &[
    SettingsSectionMeta {
        kind: ServerSettingsSection::Overview,
        label: "Обзор",
        description: "Название, иконка и базовые сведения",
    },
    SettingsSectionMeta {
        kind: ServerSettingsSection::Invites,
        label: "Инвайты",
        description: "Ссылки входа и ограничения",
    },
    SettingsSectionMeta {
        kind: ServerSettingsSection::Members,
        label: "Участники",
        description: "Список людей и быстрые действия",
    },
    SettingsSectionMeta {
        kind: ServerSettingsSection::Roles,
        label: "Роли",
        description: "Права доступа и группы",
    },
    SettingsSectionMeta {
        kind: ServerSettingsSection::Moderation,
        label: "Модерация",
        description: "Правила, журнал и фильтры",
    },
    SettingsSectionMeta {
        kind: ServerSettingsSection::Voice,
        label: "Голос",
        description: "Качество, лимиты и поведение комнат",
    },
];

/// Renders a server settings workspace.
#[component]
pub(crate) fn ServerSettingsPage(
    server: ServerSummary,
    active_section: ServerSettingsSection,
    on_select_section: EventHandler<ServerSettingsSection>,
    on_server_updated: EventHandler<ServerSummary>,
    on_close: EventHandler<()>,
) -> Element {
    let section_label = settings_section_label(active_section);
    let section_description = settings_section_description(active_section);
    let server_name = server.name.clone();

    rsx! {
        section { class: "flex min-w-0 flex-1 bg-zinc-950/35",
            nav { class: "group/settings-nav relative z-20 flex w-[292px] shrink-0 flex-col border-r border-zinc-800/80 bg-zinc-950/60 p-4 transition-[width] duration-200 ease-out max-[1440px]:w-[76px] max-[1440px]:hover:w-[292px] max-[1440px]:focus-within:w-[292px]",
                div { class: "mb-5 min-w-0 px-1",
                    p { class: "overflow-hidden whitespace-nowrap text-[11px] font-medium uppercase tracking-[0.22em] text-zinc-600 transition-[opacity] duration-150 max-[1440px]:opacity-0 max-[1440px]:group-hover/settings-nav:opacity-100 max-[1440px]:group-focus-within/settings-nav:opacity-100", "Параметры сервера" }
                    h1 { class: "mt-2 truncate text-[18px] font-semibold tracking-[-0.03em] text-zinc-50 transition-[opacity] duration-150 max-[1440px]:opacity-0 max-[1440px]:group-hover/settings-nav:opacity-100 max-[1440px]:group-focus-within/settings-nav:opacity-100", "{server_name}" }
                }
                div { class: "space-y-1",
                    for section in SETTINGS_SECTIONS {
                        button {
                            key: "{section.label}",
                            r#type: "button",
                            class: settings_item_class(active_section == section.kind),
                            "aria-current": if active_section == section.kind { "page" } else { "false" },
                            onclick: move |_| on_select_section.call(section.kind),
                            span { class: settings_badge_class(active_section == section.kind), "{settings_section_short_label(section.kind)}" }
                            span { class: "min-w-0 flex-1 overflow-hidden transition-[opacity] duration-150 max-[1440px]:opacity-0 max-[1440px]:group-hover/settings-nav:opacity-100 max-[1440px]:group-focus-within/settings-nav:opacity-100",
                                span { class: "block truncate text-[13px] font-medium", "{section.label}" }
                                span { class: "mt-0.5 block truncate text-[11px] leading-4 text-zinc-500", "{section.description}" }
                            }
                        }
                    }
                }
            }
            div { class: "min-w-0 flex-1 overflow-y-auto",
                div { class: "flex h-[72px] items-center justify-between gap-4 border-b border-zinc-800/80 bg-zinc-950/70 px-6 backdrop-blur-xl",
                    div { class: "min-w-0",
                        p { class: "text-[11px] font-medium uppercase tracking-[0.18em] text-zinc-600", "Раздел настроек" }
                        h2 { class: "truncate text-[15px] font-semibold tracking-[-0.03em] text-zinc-50", "{section_label}" }
                    }
                    button {
                        r#type: "button",
                        class: "group relative flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/80 text-zinc-400 transition hover:border-zinc-700 hover:bg-zinc-900 hover:text-zinc-100",
                        "aria-label": "Закрыть параметры сервера",
                        onclick: move |_| on_close.call(()),
                        span { class: "pointer-events-none absolute right-0 top-[calc(100%+10px)] -translate-y-1 whitespace-nowrap rounded-xl border border-zinc-800 bg-zinc-950/95 px-3 py-1.5 text-[12px] font-medium text-zinc-200 opacity-0 shadow-[0_16px_40px_rgba(0,0,0,.45)] backdrop-blur-xl transition-[opacity,transform] duration-150 group-hover:translate-y-0 group-hover:opacity-100 group-focus-visible:translate-y-0 group-focus-visible:opacity-100", "Закрыть" }
                        svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 18 18 6M6 6l12 12" }
                        }
                    }
                }
                div { class: section_container_class(active_section),
                    match active_section {
                        ServerSettingsSection::Overview => rsx! {
                            ServerOverviewSettingsSection {
                                server: server.clone(),
                                on_server_updated,
                            }
                        },
                        ServerSettingsSection::Invites => rsx! {
                            ServerInvitesSettingsSection {
                                server_id: server.id.clone(),
                                server_name: server_name.clone(),
                            }
                        },
                        ServerSettingsSection::Members => rsx! {
                            ServerMembersSettingsSection {
                                server_id: server.id.clone(),
                                server_name: server_name.clone(),
                            }
                        },
                        ServerSettingsSection::Roles => rsx! {
                            ServerRolesSettingsSection {
                                server_id: server.id.clone(),
                                server_name: server_name.clone(),
                                is_owner: server.is_owner,
                            }
                        },
                        _ => rsx! {
                            div { class: "rounded-[20px] border border-zinc-800 bg-zinc-950/70 p-6 shadow-[0_18px_60px_rgba(0,0,0,.22)]",
                                div { class: "flex items-start justify-between gap-4",
                                    div { class: "min-w-0",
                                        h3 { class: "text-[22px] font-semibold tracking-[-0.04em] text-zinc-50", "{section_label}" }
                                        p { class: "mt-2 max-w-xl text-[13px] leading-6 text-zinc-500", "{section_description}" }
                                    }
                                    span { class: "shrink-0 rounded-full border border-accent/25 bg-accent/10 px-3 py-1 text-[11px] font-medium text-blue-200", "Настройки" }
                                }
                                div { class: "mt-6 grid gap-3 sm:grid-cols-2",
                                    div { class: "rounded-2xl border border-zinc-800 bg-zinc-900/70 p-4",
                                        p { class: "text-[12px] font-semibold text-zinc-100", "Параметры" }
                                        p { class: "mt-2 text-[12px] leading-5 text-zinc-500", "Ключевые настройки выбранной секции." }
                                    }
                                    div { class: "rounded-2xl border border-zinc-800 bg-zinc-900/70 p-4",
                                        p { class: "text-[12px] font-semibold text-zinc-100", "Быстрые действия" }
                                        p { class: "mt-2 text-[12px] leading-5 text-zinc-500", "Частые действия администратора для выбранной секции." }
                                    }
                                }
                            }
                        },
                    }
                }
            }
        }
    }
}

fn settings_item_class(active: bool) -> &'static str {
    if active {
        "flex w-full items-center gap-3 rounded-xl border border-accent/25 bg-accent/10 px-3 py-2.5 text-left text-blue-100"
    } else {
        "flex w-full items-center gap-3 rounded-xl border border-transparent px-3 py-2.5 text-left text-zinc-300 transition hover:border-zinc-800 hover:bg-zinc-900 hover:text-zinc-100"
    }
}

fn settings_badge_class(active: bool) -> &'static str {
    if active {
        "flex h-8 w-8 shrink-0 items-center justify-center rounded-xl border border-accent/25 bg-accent/15 text-[12px] font-semibold text-blue-100"
    } else {
        "flex h-8 w-8 shrink-0 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/70 text-[12px] font-semibold text-zinc-500"
    }
}

fn settings_section_short_label(section: ServerSettingsSection) -> &'static str {
    match section {
        ServerSettingsSection::Overview => "О",
        ServerSettingsSection::Invites => "И",
        ServerSettingsSection::Members => "У",
        ServerSettingsSection::Roles => "Р",
        ServerSettingsSection::Moderation => "М",
        ServerSettingsSection::Voice => "Г",
    }
}

fn settings_section_label(section: ServerSettingsSection) -> &'static str {
    match section {
        ServerSettingsSection::Overview => "Обзор",
        ServerSettingsSection::Invites => "Инвайты",
        ServerSettingsSection::Members => "Участники",
        ServerSettingsSection::Roles => "Роли",
        ServerSettingsSection::Moderation => "Модерация",
        ServerSettingsSection::Voice => "Голос",
    }
}

fn settings_section_description(section: ServerSettingsSection) -> &'static str {
    match section {
        ServerSettingsSection::Overview => {
            "Общее управление сервером: название, визуальные настройки и короткое описание."
        }
        ServerSettingsSection::Invites => {
            "Просмотр активных приглашений, лимитов использования и быстрые действия со ссылками."
        }
        ServerSettingsSection::Members => {
            "Просмотр участников, инвайтов входа и быстрые действия модерации."
        }
        ServerSettingsSection::Roles => {
            "Управление ролями, правами доступа и цветами групп на сервере."
        }
        ServerSettingsSection::Moderation => {
            "Правила, журнал событий и настройки безопасности сообщества."
        }
        ServerSettingsSection::Voice => {
            "Параметры голосовых комнат, качество соединения и лимиты участников."
        }
    }
}

fn section_container_class(section: ServerSettingsSection) -> &'static str {
    match section {
        ServerSettingsSection::Invites
        | ServerSettingsSection::Members
        | ServerSettingsSection::Roles => {
            "mx-auto min-h-[calc(100vh-72px)] w-full max-w-[1180px] px-6 py-6"
        }
        _ => "mx-auto flex min-h-[calc(100vh-72px)] w-full max-w-[920px] flex-col px-6 py-8",
    }
}
