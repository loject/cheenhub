//! Server overview settings section.

use cheenhub_contracts::rest::{ServerSummary, UpdateServerRequest};
use dioxus::prelude::*;

use crate::features::app::components::server_avatar::ServerAvatar;

use super::api;

/// Renders server name and avatar settings.
#[component]
pub(crate) fn ServerOverviewSettingsSection(
    server: ServerSummary,
    on_server_updated: EventHandler<ServerSummary>,
) -> Element {
    let mut name = use_signal(|| server.name.clone());
    let mut profile_status = use_signal(ProfileUpdateStatus::default);
    let mut avatar_status = use_signal(AvatarUpdateStatus::default);
    let trimmed_name = name().trim().to_owned();
    let has_name_changes = trimmed_name != server.name;
    let name_valid = (2..=48).contains(&trimmed_name.chars().count());
    let is_profile_busy = matches!(profile_status(), ProfileUpdateStatus::Loading);
    let is_avatar_busy = matches!(avatar_status(), AvatarUpdateStatus::Loading);
    let can_edit = server.is_owner;
    let avatar_server_id = server.id.clone();
    let profile_server_id = server.id.clone();

    rsx! {
        div { class: "space-y-4",
            div { class: "rounded-[20px] border border-zinc-800 bg-zinc-950/70 p-5 shadow-[0_18px_60px_rgba(0,0,0,.22)]",
                div { class: "flex flex-col gap-4 sm:flex-row sm:items-center",
                    ServerAvatar {
                        name: server.name.clone(),
                        avatar_url: server.avatar_url.clone(),
                        class: "flex h-20 w-20 shrink-0 items-center justify-center overflow-hidden rounded-2xl border border-zinc-800 bg-accent text-[24px] font-bold text-white shadow-[0_14px_36px_rgba(59,130,246,.20)]".to_owned(),
                    }
                    div { class: "min-w-0 flex-1",
                        h3 { class: "text-[16px] font-semibold tracking-[-0.03em] text-zinc-50", "Иконка сервера" }
                        p { class: "mt-1 text-[12px] leading-5 text-zinc-500",
                            if can_edit {
                                "PNG, JPG, GIF или WebP до 8 МБ. Изображение будет обрезано в квадрат."
                            } else {
                                "Иконку может изменить только владелец сервера."
                            }
                        }
                        if can_edit {
                            div { class: "mt-3 flex flex-wrap items-center gap-2",
                                label {
                                    class: "inline-flex h-9 cursor-pointer items-center justify-center rounded-xl border border-accent/25 bg-accent/10 px-3 text-[12px] font-medium text-blue-100 transition hover:border-accent/45 hover:bg-accent/15",
                                    input {
                                        class: "sr-only",
                                        r#type: "file",
                                        accept: "image/png,image/jpeg,image/gif,image/webp,image/*",
                                        disabled: is_avatar_busy,
                                        onchange: move |event| {
                                            if is_avatar_busy {
                                                return;
                                            }
                                            let Some(file) = event.files().into_iter().next() else {
                                                return;
                                            };
                                            if file.size() > 8 * 1024 * 1024 {
                                                avatar_status.set(AvatarUpdateStatus::Failed(
                                                    "Изображение слишком большое. Загрузи файл до 8 МБ.".to_owned(),
                                                ));
                                                return;
                                            }

                                            avatar_status.set(AvatarUpdateStatus::Loading);
                                            let request_server_id = avatar_server_id.clone();
                                            info!(
                                                server_id = %request_server_id,
                                                file_name = %file.name(),
                                                file_size = file.size(),
                                                "uploading server avatar"
                                            );
                                            spawn(async move {
                                                match file.read_bytes().await {
                                                    Ok(bytes) => match api::update_server_avatar(
                                                        request_server_id.clone(),
                                                        bytes.to_vec(),
                                                    )
                                                    .await
                                                    {
                                                        Ok(updated_server) => {
                                                            info!(
                                                                server_id = %updated_server.id,
                                                                "server avatar updated"
                                                            );
                                                            on_server_updated.call(updated_server);
                                                            avatar_status.set(AvatarUpdateStatus::Succeeded);
                                                        }
                                                        Err(error) => {
                                                            warn!(
                                                                server_id = %request_server_id,
                                                                %error,
                                                                "server avatar update failed"
                                                            );
                                                            avatar_status.set(AvatarUpdateStatus::Failed(error));
                                                        }
                                                    },
                                                    Err(error) => {
                                                        warn!(?error, "failed to read selected server avatar file");
                                                        avatar_status.set(AvatarUpdateStatus::Failed(
                                                            "Не удалось прочитать выбранный файл.".to_owned(),
                                                        ));
                                                    }
                                                }
                                            });
                                        },
                                    }
                                    if is_avatar_busy { "Загружаем..." } else { "Загрузить иконку" }
                                }
                            }
                        }
                        match avatar_status() {
                            AvatarUpdateStatus::Idle => rsx! {},
                            AvatarUpdateStatus::Loading => rsx! {
                                p { class: "mt-2 text-[12px] leading-5 text-zinc-400", "Обрабатываем изображение..." }
                            },
                            AvatarUpdateStatus::Succeeded => rsx! {
                                p { class: "mt-2 text-[12px] leading-5 text-emerald-200", "Иконка сервера обновлена." }
                            },
                            AvatarUpdateStatus::Failed(error) => rsx! {
                                p { class: "mt-2 rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-2 text-[12px] leading-5 text-red-200", "{error}" }
                            },
                        }
                    }
                }
            }

            div { class: "rounded-[20px] border border-zinc-800 bg-zinc-950/70 p-5 shadow-[0_18px_60px_rgba(0,0,0,.22)]",
                h3 { class: "text-[16px] font-semibold tracking-[-0.03em] text-zinc-50", "Данные сервера" }
                p { class: "mt-1 text-[12px] leading-5 text-zinc-500",
                    if can_edit {
                        "Название видно в списке серверов, боковой панели и приглашениях."
                    } else {
                        "Название может изменить только владелец сервера."
                    }
                }
                div { class: "mt-4 space-y-3",
                    label { class: "block",
                        span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "Название" }
                        input {
                            r#type: "text",
                            value: name(),
                            maxlength: "48",
                            autocomplete: "off",
                            readonly: !can_edit,
                            oninput: move |event| {
                                name.set(event.value());
                                if !matches!(profile_status(), ProfileUpdateStatus::Idle | ProfileUpdateStatus::Loading) {
                                    profile_status.set(ProfileUpdateStatus::Idle);
                                }
                            },
                            class: "h-11 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[14px] text-zinc-100 outline-none transition placeholder:text-zinc-700 focus:border-accent/70 focus:ring-4 focus:ring-accent/10 read-only:text-zinc-500"
                        }
                        p { class: "mt-1.5 text-[11px] leading-4 text-zinc-500",
                            "2-48 символов."
                        }
                        if has_name_changes && !name_valid {
                            p { class: "mt-1 text-[11px] leading-4 text-red-200",
                                "Исправь название, чтобы сохранить изменения."
                            }
                        }
                    }
                }

                match profile_status() {
                    ProfileUpdateStatus::Idle => rsx! {},
                    ProfileUpdateStatus::Loading => rsx! {
                        p { class: "mt-3 text-[12px] leading-5 text-zinc-400", "Сохраняем настройки сервера..." }
                    },
                    ProfileUpdateStatus::Succeeded => rsx! {
                        p { class: "mt-3 text-[12px] leading-5 text-emerald-200", "Название сервера обновлено." }
                    },
                    ProfileUpdateStatus::Failed(error) => rsx! {
                        p { class: "mt-3 rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-2 text-[12px] leading-5 text-red-200", "{error}" }
                    },
                }

                if can_edit {
                    div { class: "mt-4 flex justify-end",
                        button {
                            r#type: "button",
                            disabled: is_profile_busy || !has_name_changes || !name_valid,
                            class: "flex h-10 items-center justify-center rounded-xl bg-accent px-4 text-[13px] font-semibold text-white shadow-[0_0_0_1px_rgba(59,130,246,0.3),0_8px_28px_rgba(59,130,246,0.18)] transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:bg-blue-400 disabled:cursor-not-allowed disabled:opacity-60",
                            onclick: move |_| {
                                if is_profile_busy || !has_name_changes || !name_valid {
                                    return;
                                }

                                let request_server_id = profile_server_id.clone();
                                let request = UpdateServerRequest {
                                    name: trimmed_name.clone(),
                                };
                                profile_status.set(ProfileUpdateStatus::Loading);
                                info!(server_id = %request_server_id, "updating server profile");
                                spawn(async move {
                                    match api::update_server(request_server_id.clone(), request).await {
                                        Ok(updated_server) => {
                                            info!(server_id = %updated_server.id, "server profile updated");
                                            name.set(updated_server.name.clone());
                                            on_server_updated.call(updated_server);
                                            profile_status.set(ProfileUpdateStatus::Succeeded);
                                        }
                                        Err(error) => {
                                            warn!(
                                                server_id = %request_server_id,
                                                %error,
                                                "server profile update failed"
                                            );
                                            profile_status.set(ProfileUpdateStatus::Failed(error));
                                        }
                                    }
                                });
                            },
                            if is_profile_busy { "Сохраняем..." } else { "Сохранить" }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone, PartialEq, Default)]
enum ProfileUpdateStatus {
    #[default]
    Idle,
    Loading,
    Succeeded,
    Failed(String),
}

#[derive(Clone, PartialEq, Default)]
enum AvatarUpdateStatus {
    #[default]
    Idle,
    Loading,
    Succeeded,
    Failed(String),
}
