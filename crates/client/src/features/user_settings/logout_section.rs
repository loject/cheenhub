//! User logout settings section.

use dioxus::prelude::*;

use crate::Route;
use crate::features::app::current_user::CurrentUserContext;
use crate::features::auth::api;
use crate::features::toast::ToastHandle;

/// Renders the sign-out action area.
#[component]
pub(crate) fn LogoutSettingsSection() -> Element {
    let navigator = use_navigator();
    let toast = use_context::<ToastHandle>();
    let current_user = use_context::<CurrentUserContext>().require_user();
    let user_id = current_user.id.clone();
    let mut is_logging_out = use_signal(|| false);

    rsx! {
        div { class: "rounded-2xl border border-red-500/20 bg-red-500/10 p-4",
            h3 { class: "text-[16px] font-semibold tracking-[-0.03em] text-red-100", "Выйти из аккаунта" }
            p { class: "mt-1 max-w-xl text-[12px] leading-5 text-red-100/70", "Заверши текущий сеанс на этом устройстве." }
            div { class: "mt-4 flex flex-col gap-2 sm:flex-row",
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
                        info!(user_id = %logout_user_id, "logging out current user from settings");
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
                            let _ = navigator.replace(Route::Login {});
                        });
                    },
                    if is_logging_out() { "Выходим..." } else { "Выйти" }
                }
            }
        }
    }
}

fn logout_button_class(is_logging_out: bool) -> &'static str {
    if is_logging_out {
        "flex h-10 w-full cursor-wait items-center justify-center rounded-xl bg-red-500/70 px-4 text-[12px] font-semibold text-white transition sm:h-9 sm:w-auto"
    } else {
        "flex h-10 w-full items-center justify-center rounded-xl bg-red-500 px-4 text-[12px] font-semibold text-white transition hover:bg-red-400 sm:h-9 sm:w-auto"
    }
}
