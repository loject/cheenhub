//! Строка собственной ссылки-приглашения в меню создания ссылок.

use dioxus::prelude::*;

use super::invite_link_data::OwnInviteLink;

/// Намерение пользователя по строке собственной ссылки-приглашения.
#[derive(Clone, PartialEq)]
pub(crate) enum OwnInviteLinkAction {
    /// Скопировать ссылку приглашения в буфер обмена.
    Copy {
        /// Код приглашения, для которого строится ссылка.
        code: String,
    },
    /// Удалить ссылку приглашения.
    Delete {
        /// Код удаляемого приглашения.
        code: String,
    },
}

/// Отрисовывает одну созданную пользователем ссылку-приглашение.
#[component]
pub(crate) fn OwnInviteLinkItem(
    link: OwnInviteLink,
    is_deleting: bool,
    on_action: EventHandler<OwnInviteLinkAction>,
) -> Element {
    let mut is_copied = use_signal(|| false);
    let mut copy_generation = use_signal(|| 0_u64);
    let copy_code = link.code.clone();
    let delete_code = link.code.clone();
    let copy_icon_class = if is_copied() {
        "opacity-0"
    } else {
        "opacity-100"
    };
    let check_icon_class = if is_copied() {
        "opacity-100"
    } else {
        "opacity-0"
    };

    rsx! {
        div { class: "flex items-center gap-3 rounded-xl border border-zinc-800 bg-zinc-950/70 p-2.5",
            div { class: "relative min-w-0 flex-1",
                p { class: "truncate font-mono text-[12px] text-zinc-200", "{link.code}" }
                div { class: "mt-1 flex flex-wrap items-center gap-x-2 gap-y-1 text-[11px] text-zinc-500",
                    span { "{status_label(link.is_active)}" }
                    span { class: "text-zinc-700", "•" }
                    span { "{usage_label(link.uses, link.max_uses)}" }
                    if let Some(expires_at) = link.expires_at.as_ref() {
                        span { class: "text-zinc-700", "•" }
                        span { "до {expires_at}" }
                    }
                }
            }
            button {
                r#type: "button",
                class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-lg border border-zinc-800 bg-zinc-900 text-zinc-400 transition hover:border-zinc-700 hover:text-zinc-100",
                "aria-label": "Скопировать ссылку приглашения",
                onclick: move |_| {
                    let code = copy_code.clone();
                    on_action.call(OwnInviteLinkAction::Copy { code });
                    let next_generation = copy_generation() + 1;
                    copy_generation.set(next_generation);
                    is_copied.set(true);
                    spawn(async move {
                        crate::features::runtime::sleep_ms(1600).await;
                        if *copy_generation.peek() == next_generation {
                            is_copied.set(false);
                        }
                    });
                },
                span { class: "absolute inset-0 flex items-center justify-center transition-opacity duration-200 {copy_icon_class}", "aria-hidden": "true",
                    svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24",
                        rect { x: "8", y: "8", width: "11", height: "11", rx: "2", ry: "2" }
                        path { d: "M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" }
                    }
                }
                span { class: "absolute inset-0 flex items-center justify-center transition-opacity duration-200 {check_icon_class}", "aria-hidden": "true",
                    svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2.2", view_box: "0 0 24 24",
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M20 6 9 17l-5-5" }
                    }
                }
            }
            button {
                r#type: "button",
                disabled: is_deleting,
                class: "flex h-9 shrink-0 items-center justify-center rounded-lg border border-red-500/25 bg-red-500/10 px-3 text-[12px] font-medium text-red-200 transition hover:border-red-500/40 hover:bg-red-500/15 disabled:cursor-not-allowed disabled:opacity-60",
                onclick: move |_| {
                    let code = delete_code.clone();
                    on_action.call(OwnInviteLinkAction::Delete { code });
                },
                if is_deleting { "Удаляем..." } else { "Удалить" }
            }
        }
    }
}

/// Подпись текущего состояния ссылки для пользователя.
fn status_label(is_active: bool) -> String {
    if is_active {
        "Действует".to_owned()
    } else {
        "Больше не действует".to_owned()
    }
}

/// Подпись использований ссылки с учетом лимита.
fn usage_label(uses: u32, max_uses: Option<u32>) -> String {
    match max_uses {
        Some(limit) => format!("{uses} из {limit} входов"),
        None => format!("{uses} входов"),
    }
}
