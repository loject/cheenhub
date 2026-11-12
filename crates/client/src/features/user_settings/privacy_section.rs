//! Настройки входящих контактов и список заблокированных пользователей.

use cheenhub_contracts::rest::{
    DirectMessageAudience, IncomingContactAudience, UserPrivacySettings,
};
use dioxus::prelude::*;

use crate::features::privacy::api;
use crate::features::toast::ToastHandle;

use super::styles::{primary_button_class, select_class};

/// Редактирует серверные правила приватности и управляет блокировками.
#[component]
pub(super) fn PrivacySettingsSection() -> Element {
    let toast = use_context::<ToastHandle>();
    let mut settings_resource = use_resource(api::get_settings);
    let mut blocks_resource = use_resource(api::list_blocked_users);
    let settings_result = settings_resource.read().clone();
    let blocks_result = blocks_resource.read().clone();
    let mut draft = use_signal(|| None::<UserPrivacySettings>);
    let mut saving = use_signal(|| false);
    let mut pending_unblock = use_signal(|| None::<String>);

    use_effect(move || {
        if draft().is_none()
            && let Some(Ok(settings)) = settings_resource.read().as_ref()
        {
            draft.set(Some(*settings));
        }
    });

    let displayed = draft().or_else(|| {
        settings_result
            .as_ref()
            .and_then(|result| result.as_ref().ok())
            .copied()
    });

    rsx! {
        div { class: "space-y-5",
            section { class: "rounded-2xl border border-zinc-800 bg-zinc-950/70 p-4 sm:p-5",
                h3 { class: "text-[16px] font-semibold text-zinc-50", "Входящие обращения" }
                p { class: "mt-1 text-[12px] leading-5 text-zinc-500",
                    "Выберите, кто может писать вам, звонить и приглашать в голосовые комнаты. Блокировка всегда имеет приоритет над этими правилами."
                }
                if let Some(settings) = displayed {
                    div { class: "mt-5 space-y-4",
                        div {
                            label { class: "mb-1.5 block text-[12px] font-medium text-zinc-200", "Личные сообщения" }
                            select {
                                class: select_class(),
                                value: match settings.direct_messages { DirectMessageAudience::Everyone => "everyone", DirectMessageAudience::Friends => "friends" },
                                disabled: saving(),
                                onchange: move |event| {
                                    if let Some(mut next) = draft() {
                                        next.direct_messages = match event.value().as_str() {
                                            "friends" => DirectMessageAudience::Friends,
                                            _ => DirectMessageAudience::Everyone,
                                        };
                                        draft.set(Some(next));
                                    }
                                },
                                option { value: "everyone", "Все пользователи" }
                                option { value: "friends", "Только друзья" }
                            }
                            p { class: "mt-1.5 text-[11px] leading-4 text-zinc-500",
                                "При выборе «Только друзья» прежние переписки доступны для чтения. Собеседник сможет ответить, если вы сами напишете ему после ограничения."
                            }
                        }
                        div {
                            label { class: "mb-1.5 block text-[12px] font-medium text-zinc-200", "Личные звонки" }
                            select {
                                class: select_class(),
                                value: incoming_value(settings.direct_calls),
                                disabled: saving(),
                                onchange: move |event| {
                                    if let Some(mut next) = draft() {
                                        next.direct_calls = parse_incoming(&event.value());
                                        draft.set(Some(next));
                                    }
                                },
                                option { value: "everyone", "Все пользователи" }
                                option { value: "friends", "Только друзья" }
                                option { value: "nobody", "Никто" }
                            }
                        }
                        div {
                            label { class: "mb-1.5 block text-[12px] font-medium text-zinc-200", "Приглашения в голосовые комнаты" }
                            select {
                                class: select_class(),
                                value: incoming_value(settings.voice_room_invites),
                                disabled: saving(),
                                onchange: move |event| {
                                    if let Some(mut next) = draft() {
                                        next.voice_room_invites = parse_incoming(&event.value());
                                        draft.set(Some(next));
                                    }
                                },
                                option { value: "everyone", "Все пользователи" }
                                option { value: "friends", "Только друзья" }
                                option { value: "nobody", "Никто" }
                            }
                        }
                        div { class: "flex justify-end",
                            button {
                                r#type: "button",
                                class: primary_button_class(),
                                disabled: saving(),
                                onclick: move |_| {
                                    let Some(settings) = draft() else { return; };
                                    if saving() { return; }
                                    saving.set(true);
                                    spawn(async move {
                                        match api::update_settings(settings).await {
                                            Ok(saved) => {
                                                draft.set(Some(saved));
                                                toast.success("Настройки приватности сохранены.");
                                                settings_resource.clear();
                                                settings_resource.restart();
                                            }
                                            Err(error) => toast.error(error),
                                        }
                                        saving.set(false);
                                    });
                                },
                                if saving() { "Сохраняем..." } else { "Сохранить настройки" }
                            }
                        }
                    }
                } else if let Some(Err(error)) = settings_result {
                    div { class: "mt-4 rounded-xl border border-red-500/25 bg-red-500/10 p-3 text-[12px] text-red-200",
                        p { "{error}" }
                        button {
                            class: "mt-2 underline",
                            onclick: move |_| { settings_resource.clear(); settings_resource.restart(); },
                            "Повторить"
                        }
                    }
                } else {
                    p { class: "mt-4 text-[12px] text-zinc-500", "Загрузка настроек..." }
                }
            }

            section { class: "rounded-2xl border border-zinc-800 bg-zinc-950/70 p-4 sm:p-5",
                h3 { class: "text-[16px] font-semibold text-zinc-50", "Заблокированные пользователи" }
                p { class: "mt-1 text-[12px] leading-5 text-zinc-500",
                    "Блокировка отменяет дружбу, входящие обращения и текущие личные звонки. Разблокировка не восстанавливает дружбу автоматически."
                }
                match blocks_result {
                    None => rsx! {
                        p { class: "mt-4 text-[12px] text-zinc-500", "Загрузка блокировок..." }
                    },
                    Some(Err(error)) => rsx! {
                        div { class: "mt-4 text-[12px] text-red-200",
                            p { "{error}" }
                            button {
                                class: "mt-2 underline",
                                onclick: move |_| { blocks_resource.clear(); blocks_resource.restart(); },
                                "Повторить"
                            }
                        }
                    },
                    Some(Ok(users)) => rsx! {
                        if users.is_empty() {
                            p { class: "mt-4 rounded-xl border border-zinc-800 bg-zinc-900/40 p-4 text-[12px] text-zinc-500",
                                "Здесь пока никого нет. Заблокировать пользователя можно в его меню личного диалога или через поиск друзей."
                            }
                        } else {
                            div { class: "mt-4 space-y-2",
                                for user in users {
                                    div {
                                        key: "{user.user_id}",
                                        class: "flex items-center justify-between gap-3 rounded-xl border border-zinc-800 bg-zinc-900/45 p-3",
                                        div { class: "min-w-0",
                                            p { class: "truncate text-[13px] font-medium text-zinc-100", "{user.nickname}" }
                                        }
                                        button {
                                            r#type: "button",
                                            disabled: pending_unblock().is_some(),
                                            class: "shrink-0 rounded-xl border border-zinc-700 px-3 py-2 text-[12px] text-zinc-200 hover:bg-zinc-800 disabled:opacity-50",
                                            onclick: {
                                                let user_id = user.user_id.clone();
                                                move |_| {
                                                    if pending_unblock().is_some() { return; }
                                                    pending_unblock.set(Some(user_id.clone()));
                                                    let user_id = user_id.clone();
                                                    spawn(async move {
                                                        match api::unblock_user(&user_id).await {
                                                            Ok(()) => {
                                                                toast.success("Пользователь разблокирован.");
                                                                blocks_resource.clear();
                                                                blocks_resource.restart();
                                                            }
                                                            Err(error) => toast.error(error),
                                                        }
                                                        pending_unblock.set(None);
                                                    });
                                                }
                                            },
                                            if pending_unblock().as_deref() == Some(user.user_id.as_str()) { "Снимаем..." } else { "Разблокировать" }
                                        }
                                    }
                                }
                            }
                        }
                    },
                }
            }
        }
    }
}

fn incoming_value(value: IncomingContactAudience) -> &'static str {
    match value {
        IncomingContactAudience::Everyone => "everyone",
        IncomingContactAudience::Friends => "friends",
        IncomingContactAudience::Nobody => "nobody",
    }
}

fn parse_incoming(value: &str) -> IncomingContactAudience {
    match value {
        "everyone" => IncomingContactAudience::Everyone,
        "nobody" => IncomingContactAudience::Nobody,
        _ => IncomingContactAudience::Friends,
    }
}
