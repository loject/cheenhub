//! Предварительный просмотр ожидающего изображения в форме личного сообщения.

use dioxus::prelude::*;

use super::direct_message_pending_image::{PendingDirectMessageImage, format_attachment_size};

/// Показывает thumbnail, имя, размер и удаление ожидающего изображения.
#[component]
pub(super) fn DirectMessageAttachmentPreview(
    attachment: PendingDirectMessageImage,
    busy: bool,
    on_remove: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "flex min-w-0 items-center gap-3 rounded-2xl border border-blue-400/25 bg-blue-500/10 px-3 py-2 text-zinc-100 shadow-[0_8px_22px_rgba(0,0,0,0.16)]",
            img { class: "size-12 shrink-0 rounded-xl border border-blue-300/10 object-cover", src: "{attachment.preview_data_url}", alt: "Предварительный просмотр вложения" }
            div { class: "min-w-0 flex-1",
                p { class: "truncate text-[12px] font-medium", title: "{attachment.display_name}", "{attachment.display_name}" }
                p { class: "mt-0.5 truncate text-[11px] text-zinc-400", role: "status", "aria-live": "polite", "{format_attachment_size(attachment.byte_size)}" if busy { " · Отправляем…" } else { " · Будет отправлено вместе с сообщением" } }
            }
            button {
                r#type: "button",
                disabled: busy,
                class: "flex size-10 shrink-0 items-center justify-center rounded-xl text-zinc-400 transition-colors hover:bg-white/10 hover:text-white focus-visible:bg-white/10 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-300/80 focus-visible:ring-offset-2 focus-visible:ring-offset-zinc-950 disabled:cursor-not-allowed disabled:opacity-45",
                "aria-label": "Удалить вложение",
                title: "Удалить вложение",
                onclick: move |_| on_remove.call(()),
                svg { class: "size-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true", path { stroke_linecap: "round", stroke_linejoin: "round", d: "m6 6 12 12M18 6 6 18" } }
            }
        }
    }
}
