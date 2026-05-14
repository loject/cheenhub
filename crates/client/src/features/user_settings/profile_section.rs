//! User profile settings section.

use cheenhub_contracts::rest::UpdateCurrentUserRequest;
use dioxus::prelude::*;

use crate::features::app::components::avatar::{UserAvatar, use_avatar_seed};
use crate::features::app::current_user::CurrentUserContext;
use crate::features::auth::api::{self, LinkedAccount};

use super::styles::{input_class, primary_button_class};

/// Renders profile and account controls.
#[component]
pub(crate) fn ProfileSettingsSection() -> Element {
    let current_user_context = use_context::<CurrentUserContext>();
    let current_user = current_user_context.require_user();
    use_avatar_seed(current_user.id.clone());
    let mut nickname = use_signal(|| current_user.nickname.clone());
    let mut profile_status = use_signal(ProfileUpdateStatus::default);
    let mut avatar_status = use_signal(AvatarUpdateStatus::default);
    let trimmed_nickname = nickname().trim().to_owned();
    let is_profile_busy = matches!(profile_status(), ProfileUpdateStatus::Loading);
    let has_changes = trimmed_nickname != current_user.nickname;
    let nickname_valid = is_valid_nickname(&trimmed_nickname);
    let is_avatar_busy = matches!(avatar_status(), AvatarUpdateStatus::Loading);
    let mut link_status = use_signal(String::new);
    let mut link_busy = use_signal(|| false);
    let mut unlinking_provider = use_signal(|| None::<String>);
    let mut linked_accounts_resource = use_resource(api::linked_accounts);
    let linked_accounts = linked_accounts_resource.read().clone();

    rsx! {
        form { class: "space-y-4",
            div { class: "rounded-2xl border border-zinc-800 bg-zinc-950/70 p-4",
                div { class: "flex flex-col gap-4 sm:flex-row sm:items-center",
                    UserAvatar {
                        nickname: current_user.nickname.clone(),
                        avatar_url: current_user.avatar_url.clone(),
                        class: "flex h-20 w-20 shrink-0 items-center justify-center rounded-2xl bg-accent text-[28px] font-bold text-white shadow-[0_14px_36px_rgba(59,130,246,.20)]".to_owned(),
                    }
                    div { class: "min-w-0 flex-1",
                        h3 { class: "text-[16px] font-semibold tracking-[-0.03em] text-zinc-50", "Аватар" }
                        p { class: "mt-1 text-[12px] leading-5 text-zinc-500", "PNG, JPG, GIF или WebP до 8 МБ. После загрузки изображение станет квадратным 512×512." }
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
                                        info!(
                                            file_name = %file.name(),
                                            file_size = file.size(),
                                            "uploading current user avatar"
                                        );
                                        spawn(async move {
                                            match file.read_bytes().await {
                                                Ok(bytes) => match api::update_current_user_avatar(bytes.to_vec()).await {
                                                    Ok(updated_user) => {
                                                        info!("current user avatar updated");
                                                        current_user_context.set_user(updated_user);
                                                        avatar_status.set(AvatarUpdateStatus::Succeeded);
                                                    }
                                                    Err(error) => {
                                                        warn!(%error, "current user avatar update failed");
                                                        avatar_status.set(AvatarUpdateStatus::Failed(error));
                                                    }
                                                },
                                                Err(error) => {
                                                    warn!(?error, "failed to read selected avatar file");
                                                    avatar_status.set(AvatarUpdateStatus::Failed(
                                                        "Не удалось прочитать выбранный файл.".to_owned(),
                                                    ));
                                                }
                                            }
                                        });
                                    },
                                }
                                if is_avatar_busy { "Загружаем..." } else { "Загрузить аватар" }
                            }
                        }
                        match avatar_status() {
                            AvatarUpdateStatus::Idle => rsx! {},
                            AvatarUpdateStatus::Loading => rsx! {
                                p { class: "mt-2 text-[12px] leading-5 text-zinc-400", "Обрабатываем изображение..." }
                            },
                            AvatarUpdateStatus::Succeeded => rsx! {
                                p { class: "mt-2 text-[12px] leading-5 text-emerald-200", "Аватар обновлен." }
                            },
                            AvatarUpdateStatus::Failed(error) => rsx! {
                                p { class: "mt-2 rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-2 text-[12px] leading-5 text-red-200", "{error}" }
                            },
                        }
                    }
                }
            }
            div { class: "grid gap-4",
                div { class: "rounded-2xl border border-zinc-800 bg-zinc-950/70 p-4",
                    h3 { class: "text-[16px] font-semibold tracking-[-0.03em] text-zinc-50", "Данные профиля" }
                    div { class: "mt-4 space-y-3",
                        label { class: "block",
                            span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "Никнейм" }
                            input {
                                r#type: "text",
                                value: nickname(),
                                maxlength: "32",
                                autocomplete: "nickname",
                                oninput: move |event| {
                                    nickname.set(event.value());
                                    if !matches!(profile_status(), ProfileUpdateStatus::Idle | ProfileUpdateStatus::Loading) {
                                        profile_status.set(ProfileUpdateStatus::Idle);
                                    }
                                },
                                class: input_class(),
                            }
                            p { class: "mt-1.5 text-[11px] leading-4 text-zinc-500",
                                "3-32 символа: латиница, цифры или _."
                            }
                            if has_changes && !nickname_valid {
                                p { class: "mt-1 text-[11px] leading-4 text-red-200",
                                    "Исправь никнейм, чтобы сохранить изменения."
                                }
                            }
                        }
                        label { class: "block",
                            span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "Email" }
                            input {
                                r#type: "email",
                                value: current_user.email.clone(),
                                autocomplete: "email",
                                readonly: true,
                                class: input_class(),
                            }
                        }
                    }
                }
            }
            div { class: "flex justify-end",
                button {
                    r#type: "button",
                    disabled: is_profile_busy || !has_changes || !nickname_valid,
                    class: primary_button_class(),
                    onclick: move |_| {
                        if is_profile_busy || !has_changes || !nickname_valid {
                            return;
                        }

                        let request = UpdateCurrentUserRequest {
                            nickname: trimmed_nickname.clone(),
                        };
                        profile_status.set(ProfileUpdateStatus::Loading);
                        info!("updating current user nickname");
                        spawn(async move {
                            match api::update_current_user(request).await {
                                Ok(updated_user) => {
                                    info!("current user nickname updated");
                                    nickname.set(updated_user.nickname.clone());
                                    current_user_context.set_user(updated_user);
                                    profile_status.set(ProfileUpdateStatus::Succeeded);
                                }
                                Err(error) => {
                                    warn!(%error, "current user nickname update failed");
                                    profile_status.set(ProfileUpdateStatus::Failed(error));
                                }
                            }
                        });
                    },
                    if is_profile_busy { "Сохраняем..." } else { "Сохранить профиль" }
                }
            }
            div { class: "rounded-2xl border border-zinc-800 bg-zinc-950/70 p-4",
                h3 { class: "text-[16px] font-semibold tracking-[-0.03em] text-zinc-50", "Связанные аккаунты" }
                p { class: "mt-1 text-[12px] leading-5 text-zinc-500", "Подключай внешние аккаунты для входа и будущих интеграций." }
                if !link_status().is_empty() {
                    p { class: "mt-3 rounded-xl border border-red-500/20 bg-red-500/10 px-3 py-2 text-[12px] leading-5 text-red-200",
                        "{link_status()}"
                    }
                }
                div { class: "mt-4 space-y-2",
                    match linked_accounts {
                        None => rsx! {
                            div { class: "rounded-2xl border border-zinc-800 bg-zinc-900/45 p-4",
                                div { class: "flex items-center gap-3",
                                    div { class: "h-4 w-4 animate-spin rounded-full border-2 border-zinc-700 border-t-blue-300" }
                                    p { class: "text-[13px] text-zinc-400", "Загружаем связанные аккаунты..." }
                                }
                            }
                        },
                        Some(Err(error)) => rsx! {
                            div { class: "rounded-2xl border border-red-500/20 bg-red-500/10 p-4",
                                p { class: "text-[13px] font-medium text-red-100", "Не удалось загрузить связанные аккаунты" }
                                p { class: "mt-1 text-[12px] leading-5 text-red-200/80", "{error}" }
                                button {
                                    r#type: "button",
                                    class: "mt-3 flex h-9 items-center justify-center rounded-xl border border-red-400/25 bg-red-500/10 px-3 text-[12px] font-medium text-red-100",
                                    onclick: move |_| {
                                        linked_accounts_resource.clear();
                                        linked_accounts_resource.restart();
                                    },
                                    "Повторить"
                                }
                            }
                        },
                        Some(Ok(accounts)) => rsx! {
                            {linked_accounts_list(
                                accounts,
                                link_busy(),
                                unlinking_provider(),
                                EventHandler::new(move |_| {
                                    if link_busy() {
                                        return;
                                    }

                                    link_status.set(String::new());
                                    link_busy.set(true);
                                    spawn(async move {
                                        match oauth_callback_url().await {
                                            Ok(redirect_uri) => match api::start_google_account_link(redirect_uri).await {
                                                Ok(authorization_url) => {
                                                    if let Err(error) = redirect_browser(authorization_url).await {
                                                        link_status.set(error);
                                                        link_busy.set(false);
                                                    }
                                                }
                                                Err(error) => {
                                                    link_status.set(error);
                                                    link_busy.set(false);
                                                }
                                            },
                                            Err(error) => {
                                                link_status.set(error);
                                                link_busy.set(false);
                                            }
                                        }
                                    });
                                }),
                                EventHandler::new(move |provider: String| {
                                    if unlinking_provider().is_some() {
                                        return;
                                    }

                                    link_status.set(String::new());
                                    unlinking_provider.set(Some(provider.clone()));
                                    spawn(async move {
                                        match api::unlink_account(&provider).await {
                                            Ok(()) => {
                                                unlinking_provider.set(None);
                                                linked_accounts_resource.clear();
                                                linked_accounts_resource.restart();
                                            }
                                            Err(error) => {
                                                link_status.set(error);
                                                unlinking_provider.set(None);
                                            }
                                        }
                                    });
                                }),
                            )}
                        },
                    }
                }
            }
        }
    }
}

#[derive(Clone, Default, PartialEq)]
enum ProfileUpdateStatus {
    #[default]
    Idle,
    Loading,
    Succeeded,
    Failed(String),
}

#[derive(Clone, Default, PartialEq)]
enum AvatarUpdateStatus {
    #[default]
    Idle,
    Loading,
    Succeeded,
    Failed(String),
}

fn is_valid_nickname(nickname: &str) -> bool {
    let len = nickname.chars().count();
    (3..=32).contains(&len)
        && nickname
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn linked_accounts_list(
    accounts: Vec<LinkedAccount>,
    link_busy: bool,
    unlinking_provider: Option<String>,
    on_link_google: EventHandler<()>,
    on_unlink: EventHandler<String>,
) -> Element {
    let google = accounts.iter().find(|account| account.provider == "google");
    let google_unlinking = unlinking_provider.as_deref() == Some("google");

    rsx! {
        div { class: "flex flex-col gap-3 rounded-2xl border border-zinc-800 bg-zinc-900/45 p-3 sm:flex-row sm:items-center sm:justify-between",
            div { class: "min-w-0 flex items-center gap-3",
                div { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 text-[13px] font-semibold text-zinc-100", "G" }
                div { class: "min-w-0",
                    p { class: "truncate text-[13px] font-medium text-zinc-100", "Google" }
                    p { class: "mt-0.5 truncate text-[11px] text-zinc-500",
                        if let Some(account) = google {
                            "{account_description(account)}"
                        } else {
                            "Можно использовать для входа"
                        }
                    }
                }
            }
            if google.is_some() {
                button {
                    r#type: "button",
                    disabled: google_unlinking,
                    class: "flex h-9 shrink-0 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[12px] font-medium text-zinc-300 transition hover:border-red-500/35 hover:bg-red-500/10 hover:text-red-200 disabled:cursor-wait disabled:opacity-60",
                    onclick: move |_| on_unlink.call("google".to_owned()),
                    if google_unlinking { "Отключаем..." } else { "Отключить" }
                }
            } else {
                button {
                    r#type: "button",
                    disabled: link_busy,
                    class: "flex h-9 shrink-0 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[12px] font-medium text-zinc-300 transition hover:border-accent/35 hover:bg-accent/10 hover:text-blue-100 disabled:cursor-wait disabled:opacity-60",
                    onclick: move |_| on_link_google.call(()),
                    if link_busy { "Открываем..." } else { "Подключить" }
                }
            }
        }
        div { class: "group relative flex flex-col gap-3 rounded-2xl border border-zinc-800 bg-zinc-900/35 p-3 opacity-60 sm:flex-row sm:items-center sm:justify-between",
            div { class: "min-w-0 flex items-center gap-3",
                div { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 text-[13px] font-semibold text-blue-200", "D" }
                div { class: "min-w-0",
                    p { class: "truncate text-[13px] font-medium text-zinc-100", "Discord" }
                    p { class: "mt-0.5 truncate text-[11px] text-zinc-500", "Подключение аккаунта" }
                }
            }
            button {
                r#type: "button",
                disabled: true,
                class: "flex h-9 shrink-0 cursor-not-allowed items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[12px] font-medium text-zinc-500",
                "Подключить"
            }
            span { class: "pointer-events-none absolute left-1/2 top-full z-10 mt-2 w-max -translate-x-1/2 rounded-xl border border-zinc-800 bg-zinc-950/95 px-3 py-2 text-[12px] text-zinc-200 opacity-0 shadow-[0_16px_40px_rgba(0,0,0,.45)] backdrop-blur-xl transition-opacity duration-200 group-hover:opacity-100",
                "в разработке"
            }
        }
    }
}

fn account_description(account: &LinkedAccount) -> String {
    account
        .email
        .clone()
        .or_else(|| account.display_name.clone())
        .unwrap_or_else(|| "Аккаунт подключен".to_owned())
}

async fn oauth_callback_url() -> Result<String, String> {
    let origin = document::eval("return window.location.origin;")
        .join::<String>()
        .await
        .map_err(|_| "Не удалось определить адрес приложения.".to_owned())?;

    Ok(format!(
        "{}/auth/oauth/google",
        origin.trim_end_matches('/')
    ))
}

async fn redirect_browser(url: String) -> Result<(), String> {
    let eval = document::eval(
        r#"
        const url = await dioxus.recv();
        window.location.assign(url);
        return true;
        "#,
    );
    eval.send(url)
        .map_err(|_| "Не удалось открыть Google OAuth.".to_owned())?;
    eval.join::<bool>()
        .await
        .map(|_| ())
        .map_err(|_| "Браузер не разрешил открыть Google OAuth.".to_owned())
}
