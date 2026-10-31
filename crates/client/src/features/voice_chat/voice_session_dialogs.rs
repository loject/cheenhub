//! Подтверждение переключения устройства и уведомление о вытеснении голоса.

use cheenhub_contracts::realtime::VoicePresenceEvictionReason;
use dioxus::prelude::*;

use crate::Route;
use crate::features::auth::sessions_api;
use crate::features::toast::ToastHandle;

use super::state::VoiceConnectionHandle;

/// Показывает модальные окна конфликтов голосовых сессий одного аккаунта.
#[component]
pub(super) fn VoiceSessionDialogs() -> Element {
    let handle = use_context::<VoiceConnectionHandle>();
    let confirmation = (handle.join_confirmation)();
    let eviction = (handle.eviction_notice)();
    let mut revoking = use_signal(|| false);
    let toast = use_context::<ToastHandle>();
    let navigator = use_navigator();
    let dismiss_confirmation = handle.clone();
    let confirm_join = handle.clone();
    let dismiss_eviction = handle.clone();

    rsx! {
        if let Some(conflict) = confirmation {
            div {
                class: "fixed inset-0 z-[200] flex items-center justify-center bg-black/65 px-4 py-6 backdrop-blur-sm",
                section {
                    role: "dialog",
                    "aria-modal": "true",
                    "aria-label": "Подтвердить смену голосового устройства",
                    class: "relative w-full max-w-[420px] rounded-2xl border border-zinc-800 bg-zinc-950 p-5 text-zinc-100 shadow-[0_28px_90px_rgba(0,0,0,0.55)]",
                    h2 { class: "text-[16px] font-semibold text-zinc-50", "Переключить голосовое подключение?" }
                    if conflict.same_auth_session {
                        p { class: "mt-3 text-[13px] leading-5 text-zinc-400",
                            "Другая вкладка этого устройства уже подключена к голосу. При подтверждении её голосовое подключение будет отключено."
                        }
                    } else {
                        p { class: "mt-3 text-[13px] leading-5 text-zinc-400",
                            "Другое устройство вашего аккаунта находится в другом голосовом разговоре. При подтверждении оно будет отключено от голоса."
                        }
                    }
                    div { class: "mt-5 flex flex-col gap-2 sm:flex-row sm:justify-end",
                        button {
                            r#type: "button",
                            class: "h-10 rounded-xl border border-zinc-700 px-4 text-[13px] font-medium text-zinc-200 hover:bg-zinc-800",
                            onclick: move |_| dismiss_confirmation.dismiss_join_confirmation(),
                            "Отмена"
                        }
                        button {
                            r#type: "button",
                            class: "h-10 rounded-xl border border-amber-500/30 bg-amber-500/15 px-4 text-[13px] font-semibold text-amber-100 hover:bg-amber-500/25",
                            onclick: move |_| confirm_join.confirm_join_conflict(),
                            "Отключить другое подключение"
                        }
                    }
                }
            }
        }
        if let Some(reason) = eviction {
            div {
                class: "fixed inset-0 z-[200] flex items-center justify-center bg-black/65 px-4 py-6 backdrop-blur-sm",
                section {
                    role: "dialog",
                    "aria-modal": "true",
                    "aria-label": "Голосовое подключение отключено",
                    class: "relative w-full max-w-[420px] rounded-2xl border border-zinc-800 bg-zinc-950 p-5 text-zinc-100 shadow-[0_28px_90px_rgba(0,0,0,0.55)]",
                    h2 { class: "text-[16px] font-semibold text-zinc-50", "Голосовое подключение отключено" }
                    if reason == VoicePresenceEvictionReason::SameAuthSessionTakeover {
                        p { class: "mt-3 text-[13px] leading-5 text-zinc-400",
                            "Другая вкладка с вашей сессией входа заняла голосовое подключение. Ваша авторизация сохранена."
                        }
                    } else {
                        p { class: "mt-3 text-[13px] leading-5 text-zinc-400",
                            "Ваш аккаунт переключился на другой голосовой разговор с другого устройства. Ваша авторизация сохранена."
                        }
                    }
                    p { class: "mt-3 text-[12px] leading-5 text-amber-200/85",
                        "Если это были не вы, завершите все сеансы аккаунта и войдите заново."
                    }
                    div { class: "mt-5 flex flex-col gap-2 sm:flex-row sm:justify-end",
                        button {
                            r#type: "button",
                            disabled: revoking(),
                            class: "h-10 rounded-xl border border-zinc-700 px-4 text-[13px] font-medium text-zinc-200 hover:bg-zinc-800 disabled:opacity-50",
                            onclick: move |_| dismiss_eviction.dismiss_eviction_notice(),
                            "Понятно"
                        }
                        button {
                            r#type: "button",
                            disabled: revoking(),
                            class: "h-10 rounded-xl border border-red-500/30 bg-red-500/10 px-4 text-[13px] font-semibold text-red-200 hover:bg-red-500/20 disabled:opacity-50",
                            onclick: move |_| {
                                if revoking() {
                                    return;
                                }
                                revoking.set(true);
                                spawn(async move {
                                    match sessions_api::revoke_all_sessions().await {
                                        Ok(()) => {
                                            toast.success("Все сеансы завершены. Войдите заново.");
                                            let _ = navigator.replace(Route::Login { password_reset: None });
                                        }
                                        Err(error) => {
                                            warn!(%error, "failed to revoke all sessions after voice eviction");
                                            toast.error(error);
                                            revoking.set(false);
                                        }
                                    }
                                });
                            },
                            if revoking() { "Завершаем сеансы..." } else { "Это не я — выйти везде" }
                        }
                    }
                }
            }
        }
    }
}
