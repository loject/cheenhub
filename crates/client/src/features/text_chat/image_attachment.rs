//! Вид вложения-изображения в текстовом чате.

use std::rc::Rc;

use cheenhub_contracts::realtime::TextChatImageAttachment;
use dioxus::logger::tracing::warn;
use dioxus::prelude::*;

use crate::features::app::components::chat_image_viewer::{
    ChatImageViewerContext, ChatImageViewerImage,
};
use crate::features::realtime::RealtimeHandle;

use super::realtime;

/// Рендерит одно вложение-изображение текстового чата, загруженное через realtime.
#[component]
pub(super) fn ChatImageAttachment(attachment: TextChatImageAttachment) -> Element {
    let realtime = use_context::<RealtimeHandle>();
    let attachment_id = attachment.id.clone();
    let viewer = use_context::<ChatImageViewerContext>();
    let mut thumbnail_element = use_signal(|| None::<Rc<MountedData>>);
    let content_type = attachment.content_type.clone();
    let image_width = attachment.width.max(1) as f64;
    let image_height = attachment.height.max(1) as f64;
    let aspect_width = attachment.width.max(1);
    let aspect_height = attachment.height.max(1);
    let preview_scale = (520.0_f64 / image_width)
        .min(360.0_f64 / image_height)
        .min(1.0);
    let preview_width = (image_width * preview_scale).round().max(1.0) as i32;
    let image = use_resource(move || {
        let realtime = realtime.clone();
        let attachment_id = attachment_id.clone();
        async move {
            let result = realtime::load_chat_image(&realtime, attachment_id.clone()).await;
            if let Err(error) = &result {
                warn!(
                    attachment_id = %attachment_id,
                    %error,
                    "failed to load text chat image attachment"
                );
            }
            result
        }
    });

    rsx! {
        div {
            class: "mt-2 inline-block max-w-full overflow-hidden rounded-[14px] border border-zinc-700/80 bg-zinc-950/70 shadow-[0_0_0_1px_rgba(255,255,255,0.035),0_12px_32px_rgba(0,0,0,0.28)]",
            style: "width: min({preview_width}px, 100%);",
            match image.read().as_ref() {
                Some(Ok(loaded)) => {
                    let viewer_image = ChatImageViewerImage {
                        attachment_id: attachment.id.clone(),
                        content_type: loaded.content_type.clone(),
                        data_base64: loaded.data_base64.clone(),
                        width: attachment.width,
                        height: attachment.height,
                    };
                    rsx! {
                        button {
                            r#type: "button",
                            class: "group block w-full cursor-zoom-in bg-zinc-950/80 p-1 transition-colors hover:bg-zinc-900/80",
                            "aria-label": "Открыть изображение",
                            onclick: move |_| {
                                let image = viewer_image.clone();
                                let element = thumbnail_element();
                                viewer.open(image, element);
                            },
                            img {
                                onmounted: move |event| thumbnail_element.set(Some(event.data.clone())),
                                class: "block w-full rounded-[10px] object-contain transition-opacity group-hover:opacity-95",
                                style: "aspect-ratio: {aspect_width} / {aspect_height};",
                                src: "data:{loaded.content_type};base64,{loaded.data_base64}",
                                alt: "Изображение из сообщения",
                            }
                        }
                    }
                }
                Some(Err(error)) => rsx! {
                    div { class: "w-full bg-zinc-950/80 p-1",
                        div {
                            class: "flex w-full items-center justify-center rounded-[10px] bg-red-950/20 px-3 py-2 text-center text-[12px] text-red-200",
                            style: "aspect-ratio: {aspect_width} / {aspect_height};",
                            "{error}"
                        }
                    }
                },
                None => rsx! {
                    div { class: "w-full bg-zinc-950/80 p-1",
                        div {
                            class: "relative flex w-full items-center justify-center overflow-hidden rounded-[10px] bg-zinc-900/45",
                            style: "aspect-ratio: {aspect_width} / {aspect_height};",
                            div { class: "pointer-events-none absolute inset-0 -translate-x-full animate-[chat-image-shimmer_1.35s_ease-in-out_infinite] bg-gradient-to-r from-transparent via-white/10 to-transparent" }
                            div { class: "pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_center,rgba(255,255,255,0.08),transparent_42%)]" }
                            span { class: "relative z-10 h-5 w-5 animate-spin rounded-full border-2 border-zinc-700 border-t-blue-400" }
                        }
                    }
                },
            }
        }
        div { class: "sr-only", "{content_type}" }
    }
}
