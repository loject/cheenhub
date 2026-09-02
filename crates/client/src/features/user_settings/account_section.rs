//! Настройки сеанса и управления аккаунтом.

use dioxus::prelude::*;

use super::delete_account_modal::DeleteAccountModal;
use crate::Route;
use crate::features::app::current_user::CurrentUserContext;
use crate::features::auth::api;
use crate::features::toast::ToastHandle;

/// Отображает действия, связанные с текущим сеансом и аккаунтом пользователя.
#[component]
pub(crate) fn AccountSettingsSection() -> Element {
    let navigator = use_navigator();
    let toast = use_context::<ToastHandle>();
    let current_user = use_context::<CurrentUserContext>().require_user();
    let user_id = current_user.id.clone();

    let mut is_logging_out = use_signal(|| false);
    let mut delete_modal_open = use_signal(|| false);

    rsx! {
        div { class: "space-y-4",
            section {
                class: "rounded-2xl border border-zinc-800 bg-zinc-900/35 p-4 sm:p-5",

                div { class: "flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between",
                    div { class: "min-w-0",
                        p {
                            class: "text-[10px] font-medium uppercase tracking-[0.18em] text-zinc-500",
                            "Сеанс"
                        }
                        h3 {
                            class: "mt-1.5 text-[16px] font-semibold tracking-[-0.03em] text-zinc-50",
                            "Выйти из аккаунта"
                        }
                        p {
                            class: "mt-1 max-w-xl text-[12px] leading-5 text-zinc-400",
                            "Завершит текущий сеанс на этом устройстве."
                        }
                    }

                    button {
                        r#type: "button",
                        disabled: is_logging_out(),
                        class: logout_button_class(is_logging_out()),
                        onclick: move |_| {
                            if is_logging_out() {
                                return;
                            }

                            let logout_user_id = user_id.clone();
                            is_logging_out.set(true);

                            info!(
                                user_id = %logout_user_id,
                                "logging out current user from settings"
                            );

                            spawn(async move {
                                match api::logout().await {
                                    Ok(()) => {
                                        info!(
                                            user_id = %logout_user_id,
                                            "current user logged out from settings"
                                        );
                                        toast.success("Выход выполнен.");
                                    }
                                    Err(error) => {
                                        warn!(
                                            user_id = %logout_user_id,
                                            %error,
                                            "logout request failed after local session cleanup"
                                        );
                                        toast.warning(
                                            "Сессия на этом устройстве завершена. Сервер не подтвердил выход.",
                                        );
                                    }
                                }

                                let _ = navigator.replace(Route::Login {
                                    password_reset: None,
                                });
                            });
                        },
                        if is_logging_out() {
                            "Выходим..."
                        } else {
                            "Выйти"
                        }
                    }
                }
            }

            section {
                class: "rounded-2xl border border-red-500/20 bg-red-500/[0.04] p-4 sm:p-5",

                div { class: "max-w-2xl",
                    p {
                        class: "text-[10px] font-semibold uppercase tracking-[0.18em] text-red-300/70",
                        "Опасная зона"
                    }
                    h3 {
                        class: "mt-1.5 text-[16px] font-semibold tracking-[-0.03em] text-red-100",
                        "Удаление аккаунта"
                    }
                    p {
                        class: "mt-1 text-[12px] leading-5 text-zinc-400",
                        "Удаление отключит доступ к аккаунту и запустит 30-дневный период восстановления."
                    }
                }

                div {
                    class: "mt-4 flex max-w-2xl gap-3 rounded-xl border border-zinc-800/80 bg-zinc-950/45 p-3.5",

                    div {
                        class: "mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border border-red-500/20 bg-red-500/10 text-red-300",

                        svg {
                            class: "h-4 w-4",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "1.8",
                            view_box: "0 0 24 24",
                            "aria-hidden": "true",

                            path {
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                                d: "M12 9v4m0 4h.01M10.3 4.4 2.7 17.5A1.75 1.75 0 0 0 4.2 20h15.6a1.75 1.75 0 0 0 1.5-2.5L13.7 4.4a1.95 1.95 0 0 0-3.4 0Z"
                            }
                        }
                    }

                    div {
                        p {
                            class: "text-[12px] font-semibold text-zinc-200",
                            "30 дней на восстановление"
                        }
                        p {
                            class: "mt-1 text-[11px] leading-5 text-zinc-500",
                            "В течение 30 дней после удаления аккаунт можно будет восстановить. После окончания этого периода восстановление станет невозможно."
                        }
                    }
                }

                div { class: "mt-4",
                    button {
                        r#type: "button",
                        class: "flex h-10 w-full items-center justify-center rounded-xl border border-red-500/30 bg-red-500/10 px-4 text-[12px] font-semibold text-red-200 transition-[background,border-color,color,transform] duration-150 hover:-translate-y-px hover:border-red-500/45 hover:bg-red-500/15 hover:text-red-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-red-500/50 focus-visible:ring-offset-2 focus-visible:ring-offset-zinc-950 sm:h-9 sm:w-auto",
                        onclick: move |_| delete_modal_open.set(true),
                        "Удалить аккаунт"
                    }
                }
            }

            if delete_modal_open() {
                DeleteAccountModal {
                    on_close: move |_| delete_modal_open.set(false),
                    on_confirm: move |_| {
                        // На следующем этапе здесь будет вызван серверный запрос удаления аккаунта.
                        delete_modal_open.set(false);
                    },
                }
            }
        }
    }
}

fn logout_button_class(is_logging_out: bool) -> &'static str {
    if is_logging_out {
        "flex h-10 w-full shrink-0 cursor-wait items-center justify-center rounded-xl border border-zinc-800 bg-zinc-900/60 px-4 text-[12px] font-semibold text-zinc-500 sm:h-9 sm:w-auto"
    } else {
        "flex h-10 w-full shrink-0 items-center justify-center rounded-xl border border-zinc-700 bg-zinc-900/80 px-4 text-[12px] font-semibold text-zinc-200 transition-[background,border-color,color,transform] duration-150 hover:-translate-y-px hover:border-zinc-600 hover:bg-zinc-800 hover:text-zinc-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/50 focus-visible:ring-offset-2 focus-visible:ring-offset-zinc-950 sm:h-9 sm:w-auto"
    }
}
