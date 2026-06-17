//! Компонент кнопки внешнего провайдера аутентификации.

use dioxus::prelude::*;

use crate::features::auth::api;
use crate::features::auth::components::arrow_right_icon::ArrowRightIcon;
use crate::features::auth::domain::AuthProvider;

#[component]
pub(crate) fn ProviderButton(provider: AuthProvider) -> Element {
    let mut status = use_signal(String::new);
    let mut is_busy = use_signal(|| false);
    let disabled = !provider.is_available() || is_busy();

    rsx! {
        div { class: "group relative space-y-2",
            button {
                r#type: "button",
                disabled,
                aria_describedby: if provider.is_available() { None } else { Some("provider-disabled-tooltip") },
                class: provider_button_class(provider, is_busy()),
                onclick: move |_| {
                    if provider != AuthProvider::Google || is_busy() {
                        return;
                    }

                    is_busy.set(true);
                    status.set(String::new());
                    spawn(async move {
                        let result = match oauth_callback_url().await {
                            Ok(redirect_uri) => api::start_google_oauth(redirect_uri).await,
                            Err(error) => Err(error),
                        };

                        match result {
                            Ok(authorization_url) => {
                                if let Err(error) = redirect_browser(authorization_url).await {
                                    status.set(error);
                                    is_busy.set(false);
                                }
                            }
                            Err(error) => {
                                status.set(error);
                                is_busy.set(false);
                            }
                        }
                    });
                },
                span { class: "flex items-center gap-3",
                    span { class: "flex h-7 w-7 items-center justify-center rounded-lg border border-zinc-800 bg-zinc-900 font-semibold text-zinc-200",
                        "{provider.badge()}"
                    }
                    if is_busy() {
                        "Открываем Google..."
                    } else {
                        "{provider.label()}"
                    }
                }
                ArrowRightIcon { class_name: "h-4 w-4 text-zinc-600" }
            }
            if !provider.is_available() {
                span {
                    id: "provider-disabled-tooltip",
                    role: "tooltip",
                    class: "pointer-events-none absolute left-1/2 top-[calc(100%+8px)] z-20 w-max max-w-[220px] -translate-x-1/2 -translate-y-1 rounded-xl border border-zinc-800 bg-zinc-950/95 px-3 py-2 text-[12px] font-medium text-zinc-200 opacity-0 shadow-[0_16px_40px_rgba(0,0,0,.45)] backdrop-blur-xl transition-[opacity,transform] duration-150 group-hover:translate-y-0 group-hover:opacity-100 group-focus-within:translate-y-0 group-focus-within:opacity-100",
                    "Discord-вход в разработке"
                }
            } else if !status().is_empty() {
                p { class: "text-xs text-red-300", "{status()}" }
            }
        }
    }
}

fn provider_button_class(provider: AuthProvider, is_busy: bool) -> &'static str {
    if !provider.is_available() {
        "btn-g flex h-11 w-full cursor-not-allowed items-center justify-between rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[13px] font-medium text-zinc-600"
    } else if is_busy {
        "btn-g flex h-11 w-full cursor-wait items-center justify-between rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[13px] font-medium text-zinc-300"
    } else {
        "btn-g flex h-11 w-full items-center justify-between rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[13px] font-medium text-zinc-300"
    }
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
