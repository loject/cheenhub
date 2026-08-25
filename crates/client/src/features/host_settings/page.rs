//! Страница почтовых настроек хоста.

use cheenhub_contracts::rest::{EmailTransport, UpdateHostEmailSettingsRequest};
use dioxus::prelude::*;

use crate::Route;
use crate::features::clipboard::copy_text;

use super::api::{self, HostSettingsApiError};
use super::tabs::{HostSettingsTab, host_settings_tabs};

const INPUT_CLASS: &str = "h-11 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[13px] text-zinc-100 outline-none transition-[border-color,box-shadow] duration-150 placeholder:text-zinc-700 focus:border-accent/70 focus:ring-4 focus:ring-accent/10 disabled:cursor-not-allowed disabled:opacity-60";
const BUTTON_CLASS: &str = "inline-flex min-h-11 items-center justify-center rounded-xl px-4 text-[13px] font-semibold transition-[background-color,border-color,color,opacity,scale] duration-150 ease-out active:scale-[0.96] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/70 disabled:cursor-not-allowed disabled:opacity-50 disabled:active:scale-100";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
enum SaveStatus {
    #[default]
    Idle,
    Saving,
    Saved,
    Failed(String),
}

/// Рендерит почтовые настройки, доступные только владельцу хоста.
#[component]
pub(crate) fn HostEmailSettingsPage() -> Element {
    let navigator = use_navigator();
    let route = use_route::<Route>();
    let (gmail_result, callback_email) = match route {
        Route::AppHostEmailSettings { gmail, email } => (gmail, email),
        _ => (None, None),
    };
    let mut settings_resource = use_resource(api::load_email_settings);
    let settings_result = settings_resource.read().clone();
    let mut initialized = use_signal(|| false);
    let mut transport = use_signal(|| EmailTransport::Smtp);
    let mut timeout_seconds = use_signal(|| "10".to_owned());
    let mut smtp_host = use_signal(String::new);
    let mut smtp_port = use_signal(|| "587".to_owned());
    let mut smtp_username = use_signal(String::new);
    let mut smtp_password = use_signal(String::new);
    let mut smtp_from_email = use_signal(String::new);
    let mut clear_smtp_password = use_signal(|| false);
    let mut smtp_password_configured = use_signal(|| false);
    let mut gmail_client_id = use_signal(String::new);
    let mut gmail_client_secret = use_signal(String::new);
    let mut clear_gmail_client_secret = use_signal(|| false);
    let mut gmail_client_secret_configured = use_signal(|| false);
    let mut gmail_client_secret_from_environment = use_signal(|| false);
    let mut gmail_connected = use_signal(|| false);
    let mut gmail_from_email = use_signal(|| None::<String>);
    let mut redirect_uri = use_signal(String::new);
    let mut save_status = use_signal(SaveStatus::default);
    let mut gmail_busy = use_signal(|| false);
    let mut gmail_error = use_signal(|| None::<String>);
    let mut confirm_disconnect = use_signal(|| false);
    let mut copy_status = use_signal(|| None::<String>);

    use_effect(move || {
        if initialized() {
            return;
        }
        let Some(Ok(settings)) = settings_resource.read().clone() else {
            return;
        };
        info!(
            transport = ?settings.transport,
            gmail_connected = settings.gmail_connected,
            "loaded host email settings into editor"
        );
        transport.set(settings.transport);
        timeout_seconds.set(settings.email_send_timeout_seconds.to_string());
        smtp_host.set(settings.smtp_host.unwrap_or_default());
        smtp_port.set(settings.smtp_port.to_string());
        smtp_username.set(settings.smtp_username.unwrap_or_default());
        smtp_from_email.set(settings.smtp_from_email.unwrap_or_default());
        smtp_password_configured.set(settings.smtp_password_configured);
        gmail_client_id.set(settings.gmail_client_id.unwrap_or_default());
        gmail_client_secret_configured.set(settings.gmail_client_secret_configured);
        gmail_client_secret_from_environment.set(settings.gmail_client_secret_from_environment);
        gmail_connected.set(settings.gmail_connected);
        gmail_from_email.set(settings.gmail_from_email);
        redirect_uri.set(settings.gmail_oauth_redirect_uri);
        initialized.set(true);
    });

    if settings_result.is_none() {
        return rsx! {
            section { class: "grid min-w-0 flex-1 place-items-center overflow-y-auto px-5 py-10",
                div { class: "w-full max-w-4xl space-y-4", "aria-label": "Загружаем настройки хоста",
                    div { class: "h-8 w-64 animate-pulse rounded-lg bg-zinc-800/80" }
                    div { class: "h-24 animate-pulse rounded-3xl bg-zinc-900/80 shadow-[0_0_0_1px_rgba(255,255,255,0.06)]" }
                    div { class: "h-72 animate-pulse rounded-3xl bg-zinc-900/70 shadow-[0_0_0_1px_rgba(255,255,255,0.06)]" }
                }
            }
        };
    }

    if let Some(Err(error)) = settings_result {
        let forbidden = matches!(error, HostSettingsApiError::Forbidden(_));
        return rsx! {
            section { class: "grid min-w-0 flex-1 place-items-center overflow-y-auto px-5 py-10",
                div { class: "w-full max-w-lg rounded-[20px] border border-zinc-800 bg-zinc-950/70 p-6 text-center shadow-[0_18px_60px_rgba(0,0,0,.22)]",
                    div { class: "mx-auto flex size-12 items-center justify-center rounded-2xl bg-amber-400/10 text-amber-200",
                        svg { class: "size-6", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 9v4m0 4h.01M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0Z" }
                        }
                    }
                    h1 { class: "mt-4 text-balance text-2xl font-semibold tracking-[-0.03em] text-white",
                        if forbidden { "Настройки хоста недоступны" } else { "Не удалось загрузить настройки" }
                    }
                    p { class: "mt-2 text-pretty text-sm leading-6 text-zinc-400", "{error.message()}" }
                    div { class: "mt-6 flex flex-col gap-2 sm:flex-row sm:justify-center",
                        if !forbidden {
                            button {
                                r#type: "button",
                                class: "{BUTTON_CLASS} bg-accent text-white hover:bg-blue-400",
                                onclick: move |_| {
                                    initialized.set(false);
                                    settings_resource.restart();
                                },
                                "Повторить"
                            }
                        }
                        button {
                            r#type: "button",
                            class: "{BUTTON_CLASS} bg-zinc-800 text-zinc-200 hover:bg-zinc-700",
                            onclick: move |_| { navigator.push(Route::AppFriends {}); },
                            "Вернуться к друзьям"
                        }
                    }
                }
            }
        };
    }

    let saving = matches!(save_status(), SaveStatus::Saving);
    let smtp_active = transport() == EmailTransport::Smtp;
    let gmail_active = transport() == EmailTransport::GmailApi;
    let smtp_choice_class = transport_choice_class(smtp_active);
    let gmail_choice_class = transport_choice_class(gmail_active);

    rsx! {
        section { class: "min-w-0 flex-1 overflow-y-auto bg-zinc-950/35 px-4 py-6 sm:px-6",
            div { class: "mx-auto w-full max-w-[920px] pb-10",
                div { class: "flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between",
                    div {
                        p { class: "text-[11px] font-medium uppercase tracking-[0.20em] text-zinc-600", "Настройки хоста" }
                        h1 { class: "mt-1 text-balance text-[22px] font-semibold tracking-[-0.04em] text-zinc-50", "Исходящая почта" }
                        p { class: "mt-1.5 max-w-2xl text-pretty text-[13px] leading-5 text-zinc-500", "Выбери способ отправки системных писем и настрой доступ для восстановления пароля." }
                    }
                    span { class: "inline-flex min-h-8 w-fit items-center rounded-xl border border-accent/25 bg-accent/10 px-3 text-[11px] font-medium text-blue-100",
                        if smtp_active { "Выбран SMTP" } else { "Выбран Gmail API" }
                    }
                }
                {host_settings_tabs(HostSettingsTab::Email)}

                if let Some(result) = gmail_result {
                    div { class: if result == "connected" { "mt-6 rounded-xl border border-emerald-500/20 bg-emerald-500/10 px-4 py-3 text-[12px] text-emerald-100" } else { "mt-6 rounded-xl border border-red-500/20 bg-red-500/10 px-4 py-3 text-[12px] text-red-100" },
                        if result == "connected" {
                            if let Some(email) = callback_email { "Gmail {email} успешно подключён." } else { "Gmail успешно подключён." }
                        } else { "Google не удалось подключить. Проверь OAuth-настройки и повтори." }
                    }
                }

                div { class: "mt-6 rounded-[20px] border border-amber-400/20 bg-amber-400/10 p-4",
                    p { class: "text-sm font-semibold text-amber-100", "Безопасность учётных данных" }
                    p { class: "mt-1 text-pretty text-[13px] leading-5 text-amber-100/70",
                        "Ограничь доступ к базе данных и резервным копиям: в них хранятся учётные данные почтового сервиса."
                    }
                }

                form {
                    class: "mt-6 space-y-5",
                    onsubmit: move |event| {
                        event.prevent_default();
                        let Ok(timeout) = timeout_seconds().parse::<u64>() else {
                            save_status.set(SaveStatus::Failed("Таймаут должен быть целым числом от 1 до 300.".to_owned()));
                            return;
                        };
                        let selected_transport = transport();
                        let mut request = UpdateHostEmailSettingsRequest {
                            transport: Some(selected_transport),
                            email_send_timeout_seconds: Some(timeout),
                            ..Default::default()
                        };
                        match selected_transport {
                            EmailTransport::Smtp => {
                                let Ok(port) = smtp_port().parse::<u16>() else {
                                    save_status.set(SaveStatus::Failed("Укажи корректный SMTP-порт.".to_owned()));
                                    return;
                                };
                                request.smtp_host = Some(smtp_host());
                                request.smtp_port = Some(port);
                                request.smtp_username = Some(smtp_username());
                                request.smtp_password = Some(smtp_password());
                                request.clear_smtp_password = Some(clear_smtp_password());
                                request.smtp_from_email = Some(smtp_from_email());
                            }
                            EmailTransport::GmailApi => {
                                request.gmail_client_id = Some(gmail_client_id());
                                request.gmail_client_secret = Some(gmail_client_secret());
                                request.clear_gmail_client_secret = Some(clear_gmail_client_secret());
                            }
                        }
                        save_status.set(SaveStatus::Saving);
                        info!(transport = ?request.transport, "saving host email settings");
                        spawn(async move {
                            match api::update_email_settings(request).await {
                                Ok(settings) => {
                                    info!(transport = ?settings.transport, "host email settings saved");
                                    smtp_password.set(String::new());
                                    clear_smtp_password.set(false);
                                    smtp_password_configured.set(settings.smtp_password_configured);
                                    gmail_client_secret.set(String::new());
                                    clear_gmail_client_secret.set(false);
                                    gmail_client_id.set(settings.gmail_client_id.unwrap_or_default());
                                    gmail_client_secret_configured.set(settings.gmail_client_secret_configured);
                                    gmail_client_secret_from_environment.set(settings.gmail_client_secret_from_environment);
                                    gmail_connected.set(settings.gmail_connected);
                                    gmail_from_email.set(settings.gmail_from_email);
                                    redirect_uri.set(settings.gmail_oauth_redirect_uri);
                                    save_status.set(SaveStatus::Saved);
                                }
                                Err(error) => {
                                    warn!(message = %error.message(), "failed to save host email settings");
                                    save_status.set(SaveStatus::Failed(error.message().to_owned()));
                                }
                            }
                        });
                    },
                    section { class: "rounded-[20px] border border-zinc-800 bg-zinc-950/70 p-5 shadow-[0_18px_60px_rgba(0,0,0,.22)]",
                        h2 { class: "text-balance text-lg font-semibold tracking-[-0.025em] text-white", "Транспорт" }
                        p { class: "mt-1 text-pretty text-[13px] leading-5 text-zinc-500", "Изменения применяются сразу после сохранения." }
                        div { class: "mt-4 grid rounded-xl border border-zinc-800 bg-zinc-950 p-1 sm:grid-cols-2",
                            button { r#type: "button", class: "{smtp_choice_class}", onclick: move |_| transport.set(EmailTransport::Smtp), "SMTP" }
                            button { r#type: "button", class: "{gmail_choice_class}", onclick: move |_| transport.set(EmailTransport::GmailApi), "Gmail API по HTTPS" }
                        }
                        label { class: "mt-4 block max-w-xs",
                            span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "Таймаут отправки, секунды" }
                            input { class: "{INPUT_CLASS} tabular-nums", r#type: "number", min: "1", max: "300", value: timeout_seconds(), oninput: move |event| timeout_seconds.set(event.value()) }
                        }
                    }

                    if smtp_active {
                    section { class: "rounded-[20px] border border-zinc-800 bg-zinc-950/70 p-5 shadow-[0_18px_60px_rgba(0,0,0,.22)]",
                        div { class: "flex flex-wrap items-center justify-between gap-3",
                            h2 { class: "text-balance text-lg font-semibold tracking-[-0.025em] text-white", "SMTP" }
                            span { class: if smtp_password_configured() { "rounded-full bg-emerald-400/10 px-3 py-1 text-[11px] font-semibold text-emerald-200" } else { "rounded-full bg-zinc-800 px-3 py-1 text-[11px] font-semibold text-zinc-400" }, if smtp_password_configured() { "Пароль сохранён" } else { "Пароль не задан" } }
                        }
                        div { class: "mt-4 grid gap-4 sm:grid-cols-[minmax(0,1fr)_140px]",
                            label { class: "block", span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "SMTP-хост" } input { class: "{INPUT_CLASS}", value: smtp_host(), placeholder: "smtp.gmail.com", oninput: move |event| smtp_host.set(event.value()) } }
                            label { class: "block", span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "Порт" } input { class: "{INPUT_CLASS} tabular-nums", r#type: "number", min: "1", max: "65535", value: smtp_port(), oninput: move |event| smtp_port.set(event.value()) } }
                        }
                        div { class: "mt-4 grid gap-4 sm:grid-cols-2",
                            label { class: "block", span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "Имя пользователя" } input { class: "{INPUT_CLASS}", autocomplete: "username", value: smtp_username(), oninput: move |event| smtp_username.set(event.value()) } }
                            label { class: "block", span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "Адрес отправителя" } input { class: "{INPUT_CLASS}", r#type: "email", value: smtp_from_email(), placeholder: "CheenHub <mail@example.com>", oninput: move |event| smtp_from_email.set(event.value()) } }
                        }
                        label { class: "mt-4 block",
                            span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "SMTP-пароль" }
                            input { class: "{INPUT_CLASS}", r#type: "password", autocomplete: "new-password", value: smtp_password(), placeholder: if smtp_password_configured() { "************" } else { "Введи пароль" }, oninput: move |event| smtp_password.set(event.value()) }
                        }
                        if smtp_password_configured() {
                            label { class: "mt-3 flex min-h-11 cursor-pointer items-center gap-3 rounded-xl px-1 text-[12px] text-zinc-400",
                                input { class: "size-4 accent-blue-500", r#type: "checkbox", checked: clear_smtp_password(), onchange: move |event| clear_smtp_password.set(event.checked()) }
                                "Удалить сохранённый SMTP-пароль"
                            }
                        }
                    }
                    }

                    if gmail_active {
                    section { class: "rounded-[20px] border border-zinc-800 bg-zinc-950/70 p-5 shadow-[0_18px_60px_rgba(0,0,0,.22)]",
                        div { class: "flex flex-wrap items-center justify-between gap-3",
                            div { h2 { class: "text-balance text-lg font-semibold tracking-[-0.025em] text-white", "Gmail API" } p { class: "mt-1 text-pretty text-[13px] leading-5 text-zinc-500", "Письма идут через HTTPS/443 и OAuth Google." } }
                            span { class: if gmail_connected() { "rounded-full bg-emerald-400/10 px-3 py-1 text-[11px] font-semibold text-emerald-200" } else { "rounded-full bg-zinc-800 px-3 py-1 text-[11px] font-semibold text-zinc-400" }, if gmail_connected() { "Gmail подключён" } else { "Gmail не подключён" } }
                        }
                        if let Some(email) = gmail_from_email() { p { class: "mt-3 text-sm text-zinc-300", "Подключённый адрес: " strong { class: "font-semibold text-white", "{email}" } } }
                        div { class: "mt-4 grid gap-4 sm:grid-cols-2",
                            label { class: "block",
                                span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "OAuth Client ID" }
                                input { class: "{INPUT_CLASS}", value: gmail_client_id(), oninput: move |event| gmail_client_id.set(event.value()) }
                            }
                            label { class: "block",
                                span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "OAuth Client Secret" }
                                input { class: "{INPUT_CLASS}", r#type: "password", autocomplete: "new-password", value: gmail_client_secret(), placeholder: if gmail_client_secret_configured() { "************" } else { "Введи Client Secret" }, oninput: move |event| gmail_client_secret.set(event.value()) }
                            }
                        }
                        if gmail_client_secret_configured() && !gmail_client_secret_from_environment() {
                            label { class: "mt-3 flex min-h-11 cursor-pointer items-center gap-3 rounded-xl px-1 text-[12px] text-zinc-400",
                                input { class: "size-4 accent-blue-500", r#type: "checkbox", checked: clear_gmail_client_secret(), onchange: move |event| clear_gmail_client_secret.set(event.checked()) }
                                "Удалить сохранённый Client Secret"
                            }
                        }
                        div { class: "mt-4 rounded-xl border border-zinc-800 bg-zinc-950 p-4",
                            p { class: "text-[12px] font-medium text-zinc-300", "Authorized redirect URI для Google Cloud" }
                            div { class: "mt-2 flex flex-col gap-2 sm:flex-row sm:items-center",
                                code { class: "min-w-0 flex-1 overflow-x-auto rounded-xl bg-black/25 px-3 py-3 text-[12px] text-blue-200", "{redirect_uri}" }
                                button { r#type: "button", class: "{BUTTON_CLASS} shrink-0 bg-zinc-800 text-zinc-200 hover:bg-zinc-700", onclick: move |_| { let value = redirect_uri(); spawn(async move { match copy_text(value).await { Ok(()) => copy_status.set(Some("Скопировано".to_owned())), Err(error) => copy_status.set(Some(error)), } }); }, "Скопировать" }
                            }
                            if let Some(status) = copy_status() { p { class: "mt-2 text-[11px] text-zinc-500", "{status}" } }
                        }
                        div { class: "mt-4 flex flex-col gap-2 sm:flex-row",
                            button {
                                r#type: "button",
                                class: "{BUTTON_CLASS} bg-accent text-white hover:bg-blue-400",
                                disabled: gmail_busy(),
                                onclick: move |_| {
                                    gmail_busy.set(true);
                                    gmail_error.set(None);
                                    confirm_disconnect.set(false);
                                    let request = UpdateHostEmailSettingsRequest { gmail_client_id: Some(gmail_client_id()), gmail_client_secret: Some(gmail_client_secret()), clear_gmail_client_secret: Some(clear_gmail_client_secret()), ..Default::default() };
                                    info!("saving Gmail OAuth client before connection");
                                    spawn(async move {
                                        let result = async {
                                            let saved = api::update_email_settings(request).await?;
                                            gmail_client_id.set(saved.gmail_client_id.unwrap_or_default());
                                            gmail_client_secret_configured.set(saved.gmail_client_secret_configured);
                                            gmail_client_secret_from_environment.set(saved.gmail_client_secret_from_environment);
                                            gmail_client_secret.set(String::new());
                                            clear_gmail_client_secret.set(false);
                                            api::start_gmail_connection().await
                                        }.await;
                                        match result {
                                            Ok(start) => {
                                                info!("opening Gmail OAuth authorization URL");
                                                if navigator.push(start.authorization_url).is_some() {
                                                    gmail_error.set(Some("Не удалось открыть Google OAuth на этом устройстве.".to_owned()));
                                                    gmail_busy.set(false);
                                                }
                                            }
                                            Err(error) => {
                                                warn!(message = %error.message(), "failed to start Gmail OAuth connection");
                                                gmail_error.set(Some(error.message().to_owned()));
                                                gmail_busy.set(false);
                                            }
                                        }
                                    });
                                },
                                if gmail_busy() { "Открываем Google..." } else if gmail_connected() { "Переподключить Gmail" } else { "Подключить Gmail" }
                            }
                            if gmail_connected() {
                                button {
                                    r#type: "button",
                                    class: if confirm_disconnect() { "{BUTTON_CLASS} bg-red-500 text-white hover:bg-red-400" } else { "{BUTTON_CLASS} bg-zinc-800 text-zinc-300 hover:bg-zinc-700" },
                                    disabled: gmail_busy(),
                                    onclick: move |_| {
                                        if !confirm_disconnect() { confirm_disconnect.set(true); return; }
                                        gmail_busy.set(true);
                                        gmail_error.set(None);
                                        info!("disconnecting Gmail from host email settings");
                                        spawn(async move {
                                            match api::disconnect_gmail().await {
                                                Ok(settings) => { gmail_connected.set(false); gmail_from_email.set(None); confirm_disconnect.set(false); gmail_busy.set(false); redirect_uri.set(settings.gmail_oauth_redirect_uri); info!("Gmail disconnected from host email settings"); }
                                                Err(error) => { warn!(message = %error.message(), "failed to disconnect Gmail"); gmail_error.set(Some(error.message().to_owned())); gmail_busy.set(false); }
                                            }
                                        });
                                    },
                                    if confirm_disconnect() { "Подтвердить отключение" } else { "Отключить Gmail" }
                                }
                            }
                        }
                        if confirm_disconnect() { p { class: "mt-2 text-pretty text-[12px] leading-5 text-red-200", "Повторное нажатие удалит refresh token и отключит отправку через этот Gmail." } }
                        if let Some(error) = gmail_error() { p { class: "mt-3 rounded-xl bg-red-400/10 px-3 py-2 text-pretty text-[12px] leading-5 text-red-200", "{error}" } }
                    }
                    }

                    div { class: "sticky bottom-0 flex flex-col gap-3 rounded-[18px] border border-zinc-800 bg-zinc-950/95 p-4 shadow-[0_-12px_40px_rgba(0,0,0,0.28)] backdrop-blur-xl sm:flex-row sm:items-center sm:justify-between",
                        div { class: "min-h-5 text-pretty text-[12px]",
                            match save_status() {
                                SaveStatus::Idle => rsx! { span { class: "text-zinc-500", "Изменения применятся к новым письмам." } },
                                SaveStatus::Saving => rsx! { span { class: "text-blue-200", "Сохраняем..." } },
                                SaveStatus::Saved => rsx! { span { class: "text-emerald-200", "Настройки сохранены." } },
                                SaveStatus::Failed(error) => rsx! { span { class: "text-red-200", "{error}" } },
                            }
                        }
                        button { r#type: "submit", class: "{BUTTON_CLASS} bg-accent px-6 text-white shadow-[0_0_0_1px_rgba(59,130,246,0.3),0_8px_28px_rgba(59,130,246,0.18)] hover:bg-blue-400", disabled: saving, if saving { "Сохраняем..." } else { "Сохранить настройки" } }
                    }
                }
            }
        }
    }
}

fn transport_choice_class(active: bool) -> &'static str {
    if active {
        "min-h-10 rounded-lg border border-accent/25 bg-accent/10 px-4 text-[12px] font-medium text-blue-100 transition-[background-color,border-color,color,transform] duration-150 ease-out active:scale-[0.97]"
    } else {
        "min-h-10 rounded-lg border border-transparent px-4 text-[12px] font-medium text-zinc-400 transition-[background-color,border-color,color,transform] duration-150 ease-out hover:border-zinc-800 hover:bg-zinc-900 hover:text-zinc-100 active:scale-[0.97]"
    }
}
