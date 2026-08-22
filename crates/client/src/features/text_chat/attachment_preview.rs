//! Предварительный просмотр ожидающего вложения в форме сообщения.

use super::pending_attachment::{PendingImageAttachment, format_attachment_size};
use dioxus::prelude::*;

/// Показывает имя, размер и состояние изображения до отправки.
#[component]
pub(crate) fn ChatAttachmentPreview(
    attachment: PendingImageAttachment,
    busy: bool,
    on_remove: EventHandler<()>,
) -> Element {
    rsx! { div { class: "flex min-w-0 items-center gap-3 rounded-2xl border border-blue-400/25 bg-blue-500/10 px-3 py-2 text-zinc-100 shadow-[0_8px_22px_rgba(0,0,0,0.16)]",
        div { class: "flex size-12 shrink-0 items-center justify-center overflow-hidden rounded-xl border border-blue-300/10 bg-blue-400/15 text-blue-200",
            if let Some(preview_data_url) = attachment.preview_data_url.clone() {
                img { class: "size-12 object-cover", src: "{preview_data_url}", alt: "Предварительный просмотр вложения" }
            } else if busy { span { class: "size-4 animate-spin rounded-full border-2 border-blue-200/35 border-t-blue-100", "aria-label": "Подготавливаем изображение" } }
            else { svg { class: "size-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M4 16 8.586 11.414a2 2 0 0 1 2.828 0L16 16m-2-2 1.586-1.586a2 2 0 0 1 2.828 0L20 14M6 20h12a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2Z" } } }
        }
        div { class: "min-w-0 flex-1", p { class: "truncate text-[12px] font-medium", title: "{attachment.display_name}", "{attachment.display_name}" }
            p { class: "mt-0.5 truncate text-[11px] text-zinc-400", role: "status", "aria-live": "polite", "{format_attachment_size(attachment.byte_size)}" if busy { " · Подготавливаем к отправке…" } else { " · Будет отправлено вместе с сообщением" } }
        }
        button { r#type: "button", disabled: busy, class: "flex size-10 shrink-0 items-center justify-center rounded-xl text-zinc-400 transition-colors hover:bg-white/10 hover:text-white focus-visible:bg-white/10 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-300/80 focus-visible:ring-offset-2 focus-visible:ring-offset-zinc-950 disabled:cursor-not-allowed disabled:opacity-45", "aria-label": "Удалить вложение", title: "Удалить вложение", onclick: move |_| on_remove.call(()),
            svg { class: "size-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", "aria-hidden": "true", path { stroke_linecap: "round", stroke_linejoin: "round", d: "m6 6 12 12M18 6 6 18" } }
        }
    } }
}
