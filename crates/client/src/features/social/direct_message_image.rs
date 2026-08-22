//! Изображение в личном сообщении.

use std::rc::Rc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use cheenhub_contracts::rest::DmImageAttachmentSummary;
use dioxus::logger::tracing::warn;
use dioxus::prelude::*;

use crate::features::app::components::chat_image_viewer::{
    ChatImageViewerContext, ChatImageViewerImage,
};

use super::api;

/// Показывает защищённо загруженное изображение личного сообщения.
#[component]
pub(crate) fn DirectMessageImage(
    conversation_id: String,
    author_user_id: String,
    image: DmImageAttachmentSummary,
) -> Element {
    let current_user =
        use_context::<crate::features::app::current_user::CurrentUserContext>().require_user();
    let viewer = use_context::<ChatImageViewerContext>();
    let mut thumbnail_element = use_signal(|| None::<Rc<MountedData>>);
    let wrapper_class = if author_user_id == current_user.id {
        "ml-auto mt-1 max-w-[520px] overflow-hidden rounded-xl border border-blue-500/20 bg-blue-950/20 p-1"
    } else {
        "mt-1 max-w-[520px] overflow-hidden rounded-xl border border-zinc-700/80 bg-zinc-950/70 p-1"
    };
    let image_id = image.id.clone();
    let content_type = image.content_type.clone();
    let loaded = use_resource(move || {
        let conversation_id = conversation_id.clone();
        let image_id = image_id.clone();
        async move {
            let result = api::load_dm_image(&conversation_id, &image_id).await;
            if let Err(error) = &result {
                warn!(
                    image_id = %image_id,
                    %error,
                    "failed to load direct-message image attachment"
                );
            }
            result.map(|bytes| BASE64.encode(bytes))
        }
    });
    let width = image.width.max(1);
    let height = image.height.max(1);
    rsx! {
        div { class: wrapper_class,
            match loaded.read().as_ref() {
                Some(Ok(data)) => {
                    let viewer_image = ChatImageViewerImage {
                        attachment_id: image.id.clone(),
                        content_type: content_type.clone(),
                        data_base64: data.clone(),
                        width: image.width,
                        height: image.height,
                    };
                    rsx! {
                        button {
                            r#type: "button",
                            class: "group block w-full cursor-zoom-in",
                            "aria-label": "Открыть изображение из личного сообщения",
                            onclick: move |_| {
                                let image = viewer_image.clone();
                                let element = thumbnail_element();
                                viewer.open(image, element);
                            },
                            img {
                                onmounted: move |event| thumbnail_element.set(Some(event.data.clone())),
                                class: "block max-h-[360px] w-full rounded-lg object-contain transition-opacity group-hover:opacity-95",
                                style: "aspect-ratio: {width} / {height};",
                                src: "data:{content_type};base64,{data}",
                                alt: "Изображение из личного сообщения",
                            }
                        }
                    }
                }
                Some(Err(error)) => rsx! { div { class: "flex min-h-24 items-center justify-center rounded-lg bg-red-950/20 p-3 text-center text-xs text-red-200", "{error}" } },
                None => rsx! { div { class: "flex min-h-24 items-center justify-center rounded-lg bg-zinc-900/60", span { class: "h-5 w-5 animate-spin rounded-full border-2 border-zinc-700 border-t-blue-400" } } },
            }
        }
    }
}
