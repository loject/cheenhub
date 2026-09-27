//! Секция со списком собственных ссылок-приглашений и лимитом их количества.

use dioxus::prelude::*;

use super::invite_link_data::OwnInviteLinks;
use super::own_invite_link_item::{OwnInviteLinkAction, OwnInviteLinkItem};

/// Отрисовывает ссылки-приглашения текущего пользователя, лимит и состояния загрузки.
#[component]
pub(crate) fn OwnInviteLinksSection(
    links: Option<OwnInviteLinks>,
    load_error: String,
    deleting_code: String,
    on_reload: EventHandler<()>,
    on_action: EventHandler<OwnInviteLinkAction>,
) -> Element {
    let is_loading = links.is_none() && load_error.is_empty();
    let limit_reached = links.as_ref().is_some_and(|links| links.is_limit_reached());

    rsx! {
        div { class: "space-y-2 rounded-2xl border border-zinc-800 bg-zinc-900/60 p-3",
            div { class: "flex items-center justify-between gap-3",
                p { class: "text-[13px] font-medium text-zinc-100", "Твои ссылки приглашения" }
                if let Some(links) = links.as_ref() {
                    span { class: "shrink-0 rounded-full border border-zinc-800 bg-zinc-950 px-2 py-0.5 text-[11px] font-medium text-zinc-400",
                        "{links.active_count()} из {links.limit}"
                    }
                }
            }
            p { class: "mt-0.5 text-[12px] leading-5 text-zinc-500",
                "Ссылки, которые ты создал для этого сервера. Их можно копировать и удалить."
            }

            if is_loading {
                div { class: "mt-3 space-y-2",
                    for _ in 0..2 {
                        div { class: "h-[58px] animate-pulse rounded-xl border border-zinc-800 bg-zinc-950/70" }
                    }
                }
            } else if !load_error.is_empty() {
                div { class: "mt-3 space-y-2 rounded-xl border border-amber-500/20 bg-amber-500/10 p-3",
                    p { class: "text-[12px] leading-5 text-amber-100", "{load_error}" }
                    button {
                        r#type: "button",
                        class: "mt-2 h-9 w-full rounded-xl border border-amber-500/30 bg-amber-500/10 text-[12px] font-medium text-amber-100 transition hover:bg-amber-500/20",
                        onclick: move |_| on_reload.call(()),
                        "Попробовать снова"
                    }
                }
            } else if links.as_ref().is_some_and(|links| links.links.is_empty()) {
                div { class: "mt-3 rounded-xl border border-dashed border-zinc-800 bg-zinc-950/50 p-3",
                    p { class: "text-[12px] leading-5 text-zinc-400",
                        "Ты еще не создавал ссылки приглашения для этого сервера. Создай первую, чтобы пригласить людей."
                    }
                }
            } else if let Some(links) = links.as_ref() {
                if limit_reached {
                    div { class: "mt-3 rounded-xl border border-amber-500/20 bg-amber-500/10 p-3",
                        p { class: "text-[12px] leading-5 text-amber-100",
                            "Достигнут лимит ссылок приглашения. Удали ссылку, которая больше не нужна, чтобы создать новую."
                        }
                    }
                }
                div { class: "mt-3 space-y-2",
                    for link in links.links.iter() {
                        OwnInviteLinkItem {
                            key: "{link.code}",
                            link: link.clone(),
                            is_deleting: deleting_code == link.code,
                            on_action: on_action,
                        }
                    }
                }
            }
        }
    }
}
