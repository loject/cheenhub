//! Глобальный интерфейс входящего личного звонка.

use cheenhub_contracts::realtime::DirectCallSnapshot;
use dioxus::prelude::*;

use crate::features::app::components::avatar::UserAvatar;

use super::direct_call_state::DirectCallHandle;

/// Показывает входящий звонок поверх текущей рабочей области.
#[component]
pub(super) fn DirectCallPrompt(call: DirectCallSnapshot, exiting: bool) -> Element {
    let direct_call = use_context::<DirectCallHandle>();
    let busy = direct_call.busy();
    let caller_nickname = call.caller_nickname.clone();
    let caller_avatar_url = call.caller_avatar_url.clone();
    let error = direct_call.error_for_conversation(&call.conversation_id);
    let accept_call = direct_call.clone();
    let decline_call = direct_call.clone();

    rsx! {
        div {
            class: if exiting {
                "direct-call-prompt direct-call-prompt-exit fixed inset-0 z-[100] flex items-end justify-center bg-black/55 p-4 backdrop-blur-sm min-[640px]:items-center"
            } else {
                "direct-call-prompt fixed inset-0 z-[100] flex items-end justify-center bg-black/55 p-4 backdrop-blur-sm min-[640px]:items-center"
            },
            role: "dialog",
            "aria-modal": "true",
            "aria-labelledby": "incoming-direct-call-title",
            div {
                class: if exiting {
                    "direct-call-prompt-card direct-call-prompt-card-exit w-full max-w-sm rounded-[32px] bg-zinc-950/95 p-3 shadow-[0_0_0_1px_rgba(255,255,255,.10),0_28px_90px_rgba(0,0,0,.58)]"
                } else {
                    "direct-call-prompt-card w-full max-w-sm rounded-[32px] bg-zinc-950/95 p-3 shadow-[0_0_0_1px_rgba(255,255,255,.10),0_28px_90px_rgba(0,0,0,.58)]"
                },
                div {
                    class: "rounded-[20px] bg-zinc-900/80 px-5 py-6 text-center",
                    p {
                        class: "direct-call-enter-segment text-[12px] font-medium uppercase tracking-[0.16em] text-blue-300",
                        "Входящий звонок"
                    }
                    UserAvatar {
                        nickname: caller_nickname.clone(),
                        avatar_url: caller_avatar_url,
                        class: "direct-call-avatar direct-call-avatar-ringing mx-auto mt-5 flex h-24 w-24 items-center justify-center rounded-full bg-zinc-800 text-[30px] font-bold text-zinc-50 shadow-[0_0_0_1px_rgba(255,255,255,.10),0_18px_45px_rgba(0,0,0,.34)]".to_owned(),
                        avatar_seed: Some(call.caller_user_id.clone()),
                    }
                    div { class: "direct-call-enter-segment direct-call-enter-segment-copy",
                        h2 {
                            id: "incoming-direct-call-title",
                            class: "mt-4 text-balance text-[20px] font-semibold text-zinc-50",
                            "{caller_nickname}"
                        }
                        p {
                            class: "mt-1 text-pretty text-[13px] leading-5 text-zinc-400",
                            "Хочет поговорить с тобой"
                        }
                    }
                    if let Some(message) = error {
                        p {
                            class: "direct-call-status-enter mt-3 rounded-xl bg-red-500/10 px-3 py-2 text-pretty text-[12px] leading-5 text-red-200 shadow-[0_0_0_1px_rgba(248,113,113,.22)]",
                            role: "alert",
                            "{message}"
                        }
                    }
                    div { class: "direct-call-enter-segment direct-call-enter-segment-actions mt-6 grid grid-cols-2 gap-3",
                        button {
                            r#type: "button",
                            disabled: busy,
                            class: "flex min-h-12 items-center justify-center gap-2 rounded-2xl bg-red-500/12 px-4 text-[13px] font-semibold text-red-100 shadow-[0_0_0_1px_rgba(248,113,113,.28)] transition-[scale,background-color,opacity] duration-150 ease-out active:scale-[0.96] disabled:cursor-wait disabled:opacity-60",
                            onclick: move |_| decline_call.decline(),
                            svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6.6 10.8a8.3 8.3 0 0 1 10.8 0l1.7-1.7a1.5 1.5 0 0 1 2.1 0l.7.7a1.5 1.5 0 0 1 0 2.1l-2.3 2.3a2 2 0 0 1-2.2.4l-2.1-1a8.2 8.2 0 0 0-6.6 0l-2.1 1a2 2 0 0 1-2.2-.4l-2.3-2.3a1.5 1.5 0 0 1 0-2.1l.7-.7a1.5 1.5 0 0 1 2.1 0l1.7 1.7Z" }
                            }
                            "Отклонить"
                        }
                        button {
                            r#type: "button",
                            disabled: busy,
                            class: "flex min-h-12 items-center justify-center gap-2 rounded-2xl bg-accent px-4 text-[13px] font-semibold text-white shadow-[0_12px_32px_rgba(37,99,235,.22)] transition-[scale,background-color,opacity] duration-150 ease-out active:scale-[0.96] disabled:cursor-wait disabled:opacity-60",
                            onclick: move |_| accept_call.accept(),
                            svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M2.25 6.75c0 8.284 6.716 15 15 15h2.25a2.25 2.25 0 0 0 2.25-2.25v-1.372c0-.516-.351-.966-.852-1.091l-4.423-1.106c-.44-.11-.902.055-1.173.417l-.97 1.293c-.282.376-.769.542-1.21.38a12.035 12.035 0 0 1-7.143-7.143c-.162-.441.004-.928.38-1.21l1.293-.97c.362-.271.527-.734.417-1.173L6.963 3.102A1.125 1.125 0 0 0 5.872 2.25H4.5A2.25 2.25 0 0 0 2.25 4.5v2.25Z" }
                            }
                            "Ответить"
                        }
                    }
                }
            }
        }
    }
}
