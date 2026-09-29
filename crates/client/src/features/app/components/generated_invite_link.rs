//! Блок готовой ссылки приглашения с копированием в буфер обмена.

use dioxus::prelude::*;

use crate::features::clipboard::copy_text;
use crate::features::runtime::sleep_ms;
use crate::features::toast::ToastHandle;

/// Показывает созданную ссылку и копирует её по нажатию.
#[component]
pub(crate) fn GeneratedInviteLink(link: String) -> Element {
    let toast = use_context::<ToastHandle>();
    let mut is_copied = use_signal(|| false);
    let mut copy_generation = use_signal(|| 0_u64);
    let copy_icon_class = if is_copied() {
        "opacity-0"
    } else {
        "opacity-100"
    };
    let copy_icon_style = if is_copied() {
        "transform: scale(0.72) rotate(-12deg);"
    } else {
        "transform: scale(1) rotate(0deg);"
    };
    let check_icon_class = if is_copied() {
        "opacity-100"
    } else {
        "opacity-0"
    };
    let check_icon_style = if is_copied() {
        "transform: scale(1) rotate(0deg);"
    } else {
        "transform: scale(0.72) rotate(12deg);"
    };

    rsx! {
        div { class: "space-y-2 rounded-2xl border border-emerald-500/20 bg-emerald-500/10 p-3",
            span { class: "block text-[12px] font-medium text-emerald-100", "Готовая ссылка" }
            div { class: "flex gap-2",
                input {
                    r#type: "text",
                    readonly: true,
                    value: "{link}",
                    class: "h-11 min-w-0 flex-1 rounded-xl border border-emerald-500/20 bg-zinc-950 px-3 text-[13px] text-zinc-100 outline-none"
                }
                button {
                    r#type: "button",
                    class: "relative flex h-11 w-11 shrink-0 items-center justify-center rounded-xl bg-emerald-500 text-emerald-950 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:bg-emerald-400",
                    "aria-label": if is_copied() { "Ссылка скопирована" } else { "Скопировать ссылку" },
                    onclick: move |_| {
                        let link_to_copy = link.clone();
                        is_copied.set(false);
                        spawn(async move {
                            match copy_text(link_to_copy).await {
                                Ok(()) => {
                                    let next_generation = copy_generation() + 1;
                                    copy_generation.set(next_generation);
                                    is_copied.set(true);
                                    toast.success("Ссылка скопирована.");
                                    info!("copied generated server invite link");
                                    sleep_ms(1400).await;

                                    if copy_generation() == next_generation {
                                        is_copied.set(false);
                                    }
                                }
                                Err(error) => {
                                    warn!(%error, "failed to copy generated server invite link");
                                    toast.error(error);
                                }
                            }
                        });
                    },
                    span { class: "absolute inset-0 flex items-center justify-center transition-[opacity,transform] duration-200 ease-out {copy_icon_class}", style: copy_icon_style, "aria-hidden": "true",
                        svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24",
                            rect { x: "8", y: "8", width: "11", height: "11", rx: "2", ry: "2" }
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" }
                        }
                    }
                    span { class: "absolute inset-0 flex items-center justify-center transition-[opacity,transform] duration-200 ease-out {check_icon_class}", style: check_icon_style, "aria-hidden": "true",
                        svg { class: "h-5 w-5", fill: "none", stroke: "currentColor", stroke_width: "2.2", view_box: "0 0 24 24",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M20 6 9 17l-5-5" }
                        }
                    }
                }
            }
        }
    }
}
