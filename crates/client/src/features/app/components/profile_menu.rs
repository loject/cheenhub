//! Меню профиля в боковой панели приложения.

use dioxus::prelude::*;

use crate::Route;
use crate::features::app::current_user::CurrentUserContext;
use crate::features::auth::api;
use crate::features::toast::ToastHandle;

/// Рендерит действия текущего пользователя в нижнем блоке боковой панели.
#[component]
pub(crate) fn ProfileMenu(on_close: EventHandler<()>) -> Element {
    let navigator = use_navigator();
    let toast = use_context::<ToastHandle>();
    let current_user_context = use_context::<CurrentUserContext>();
    let current_user = current_user_context.require_user();
    let user_id = current_user.id.clone();
    let mut is_logging_out = use_signal(|| false);

    rsx! {
        div {
            class: "absolute bottom-[calc(100%+10px)] left-3 right-3 z-50 overflow-hidden rounded-[18px] border border-zinc-800 bg-zinc-950/96 p-1.5 shadow-[0_20px_60px_rgba(0,0,0,.58)] backdrop-blur-xl",
            onclick: move |event| event.stop_propagation(),
            div { class: "px-2.5 py-2",
                p { class: "truncate text-[13px] font-semibold text-zinc-100", "{current_user.nickname}" }
                p { class: "mt-0.5 text-[11px] text-zinc-500", "Текущий аккаунт" }
            }
            div { class: "my-1 border-t border-zinc-800/70" }
            div { class: "group relative",
                button {
                    r#type: "button",
                    disabled: true,
                    class: "flex w-full cursor-not-allowed items-center gap-2 rounded-xl px-3 py-2.5 text-left text-[13px] text-zinc-500 opacity-70",
                    "aria-describedby": "profile-menu-accounts-tooltip",
                    svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15 19.128a9.38 9.38 0 0 0 2.625.372 9.337 9.337 0 0 0 4.121-.952 4.125 4.125 0 0 0-7.533-2.493M15 19.128v-.003c0-1.113-.285-2.16-.786-3.07M15 19.128v.106A12.318 12.318 0 0 1 8.624 21c-2.331 0-4.512-.645-6.374-1.766l-.001-.109a6.375 6.375 0 0 1 11.964-3.07M12 7.5a3.75 3.75 0 1 1-7.5 0 3.75 3.75 0 0 1 7.5 0Zm6.75 0a2.625 2.625 0 1 1-5.25 0 2.625 2.625 0 0 1 5.25 0Z" }
                    }
                    span { class: "min-w-0 flex-1 truncate", "Управление аккаунтами" }
                    span { class: "rounded-full border border-zinc-800 px-2 py-0.5 text-[10px] font-medium text-zinc-600", "скоро" }
                }
                span {
                    id: "profile-menu-accounts-tooltip",
                    role: "tooltip",
                    class: "pointer-events-none absolute bottom-[calc(100%+8px)] left-2 right-2 z-50 translate-y-1 rounded-xl border border-zinc-800 bg-zinc-950/95 px-3 py-2 text-[11px] leading-4 text-zinc-300 opacity-0 shadow-[0_16px_40px_rgba(0,0,0,.45)] backdrop-blur-xl transition-[opacity,transform] duration-150 group-hover:translate-y-0 group-hover:opacity-100 group-focus-within:translate-y-0 group-focus-within:opacity-100",
                    "Управление несколькими аккаунтами пока недоступно."
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
                    info!(user_id = %logout_user_id, "logging out current user from profile menu");
                    spawn(async move {
                        match api::logout().await {
                            Ok(()) => {
                                info!(
                                    user_id = %logout_user_id,
                                    "current user logged out from profile menu"
                                );
                                toast.success("Выход выполнен.");
                            }
                            Err(error) => {
                                warn!(
                                    user_id = %logout_user_id,
                                    %error,
                                    "logout request failed after local session cleanup from profile menu"
                                );
                                toast.warning(
                                    "Сессия на этом устройстве завершена. Сервер не подтвердил выход.",
                                );
                            }
                        }
                        current_user_context.clear_user();
                        on_close.call(());
                        let _ = navigator.replace(Route::Login {});
                    });
                },
                svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15.75 9V5.25A2.25 2.25 0 0 0 13.5 3h-6a2.25 2.25 0 0 0-2.25 2.25v13.5A2.25 2.25 0 0 0 7.5 21h6a2.25 2.25 0 0 0 2.25-2.25V15m3 0 3-3m0 0-3-3m3 3H9" }
                }
                if is_logging_out() {
                    "Выходим..."
                } else {
                    "Выйти из текущего аккаунта"
                }
            }
        }
    }
}

fn logout_button_class(is_logging_out: bool) -> &'static str {
    if is_logging_out {
        "mt-1 flex w-full cursor-wait items-center gap-2 rounded-xl px-3 py-2.5 text-left text-[13px] text-red-300/60 opacity-80"
    } else {
        "mt-1 flex w-full items-center gap-2 rounded-xl px-3 py-2.5 text-left text-[13px] text-red-300 transition-[background,border-color,color,transform,opacity] duration-150 hover:bg-red-500/10 hover:text-red-200"
    }
}
