//! Настройки отображаемого имени системного автора.

use dioxus::prelude::*;

use super::api;

/// Рендерит поле системного имени сервера с загрузкой, сохранением и ошибками.
#[component]
pub(crate) fn ServerSystemAccountSettingsSection(server_id: String, is_owner: bool) -> Element {
    let load_server_id = server_id.clone();
    let profile_load = use_resource(move || {
        let request_server_id = load_server_id.clone();
        async move { api::get_system_account(request_server_id).await }
    });
    let mut saved_nickname = use_signal(|| None::<String>);
    let mut nickname = use_signal(|| None::<String>);
    let mut load_error = use_signal(String::new);
    let mut save_status = use_signal(SaveStatus::default);

    let effect_server_id = server_id.clone();
    use_effect(move || {
        if saved_nickname().is_some() || !is_owner {
            return;
        }
        let Some(result) = profile_load.read().clone() else {
            return;
        };
        match result {
            Ok(profile_nickname) => {
                info!(server_id = %effect_server_id, "loaded server system account settings");
                saved_nickname.set(Some(profile_nickname.clone()));
                nickname.set(Some(profile_nickname));
            }
            Err(error) => {
                warn!(server_id = %effect_server_id, %error, "failed to load server system account settings");
                load_error.set(error);
            }
        }
    });

    let saved = saved_nickname();
    let draft = nickname();
    let current_nickname = draft.as_deref().unwrap_or_default();
    let trimmed_nickname = current_nickname.trim().to_owned();
    let has_changes = saved.as_deref() != Some(trimmed_nickname.as_str());
    let valid = (1..=48).contains(&trimmed_nickname.chars().count());
    let is_loading = is_owner && saved.is_none() && load_error().is_empty();
    let is_saving = matches!(save_status(), SaveStatus::Loading);
    let request_server_id = server_id.clone();

    rsx! {
        div { class: "rounded-[20px] border border-zinc-800 bg-zinc-950/70 p-5 shadow-[0_18px_60px_rgba(0,0,0,.22)]",
            h3 { class: "text-[16px] font-semibold tracking-[-0.03em] text-zinc-50", "Системные сообщения" }
            p { class: "mt-1 text-[12px] leading-5 text-zinc-500", "Сообщения от сервера будут использовать это имя и иконку сервера." }
            if !is_owner {
                p { class: "mt-4 text-[12px] leading-5 text-zinc-500", "Имя системного автора может изменить только владелец сервера." }
            } else if is_loading {
                p { class: "mt-4 text-[12px] leading-5 text-zinc-400", "Загружаем настройки системного автора..." }
            } else if !load_error().is_empty() {
                p { class: "mt-4 rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-2 text-[12px] leading-5 text-red-200", "{load_error()}" }
            } else {
                label { class: "mt-4 block",
                    span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "Имя автора" }
                    input {
                        r#type: "text",
                        value: current_nickname.to_owned(),
                        maxlength: "48",
                        autocomplete: "off",
                        disabled: is_saving,
                        oninput: move |event| {
                            nickname.set(Some(event.value()));
                            if !matches!(save_status(), SaveStatus::Idle | SaveStatus::Loading) {
                                save_status.set(SaveStatus::Idle);
                            }
                        },
                        class: "h-11 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[14px] text-zinc-100 outline-none transition focus:border-accent/70 focus:ring-4 focus:ring-accent/10 disabled:text-zinc-500"
                    }
                    p { class: "mt-1.5 text-[11px] leading-4 text-zinc-500", "По умолчанию используется название сервера; можно задать отдельное имя длиной до 48 символов." }
                    if has_changes && !valid {
                        p { class: "mt-1 text-[11px] leading-4 text-red-200", "Укажи имя длиной от 1 до 48 символов." }
                    }
                }
                match save_status() {
                    SaveStatus::Idle => rsx! {},
                    SaveStatus::Loading => rsx! { p { class: "mt-3 text-[12px] leading-5 text-zinc-400", "Сохраняем имя..." } },
                    SaveStatus::Succeeded => rsx! { p { class: "mt-3 text-[12px] leading-5 text-emerald-200", "Имя системного автора обновлено." } },
                    SaveStatus::Failed(error) => rsx! { p { class: "mt-3 rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-2 text-[12px] leading-5 text-red-200", "{error}" } },
                }
                div { class: "mt-4 flex justify-end",
                    button {
                        r#type: "button",
                        disabled: is_saving || !has_changes || !valid,
                        class: "flex h-10 items-center justify-center rounded-xl bg-accent px-4 text-[13px] font-semibold text-white transition hover:bg-blue-400 disabled:cursor-not-allowed disabled:opacity-60",
                        onclick: move |_| {
                            if is_saving || !has_changes || !valid {
                                return;
                            }
                            let save_server_id = request_server_id.clone();
                            let save_nickname = trimmed_nickname.clone();
                            save_status.set(SaveStatus::Loading);
                            info!(server_id = %save_server_id, "updating server system account name");
                            spawn(async move {
                                match api::update_system_account(save_server_id.clone(), save_nickname).await {
                                    Ok(updated_nickname) => {
                                        saved_nickname.set(Some(updated_nickname.clone()));
                                        nickname.set(Some(updated_nickname));
                                        save_status.set(SaveStatus::Succeeded);
                                        info!(server_id = %save_server_id, "server system account name updated");
                                    }
                                    Err(error) => {
                                        warn!(server_id = %save_server_id, %error, "server system account name update failed");
                                        save_status.set(SaveStatus::Failed(error));
                                    }
                                }
                            });
                        },
                        if is_saving { "Сохраняем..." } else { "Сохранить имя" }
                    }
                }
            }
        }
    }
}

#[derive(Clone, PartialEq, Default)]
enum SaveStatus {
    #[default]
    Idle,
    Loading,
    Succeeded,
    Failed(String),
}
