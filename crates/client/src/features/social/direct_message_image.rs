//! Изображение в личном сообщении.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use cheenhub_contracts::rest::DmImageAttachmentSummary;
use dioxus::prelude::*;

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
            api::load_dm_image(&conversation_id, &image_id)
                .await
                .map(|bytes| BASE64.encode(bytes))
        }
    });
    let width = image.width.max(1);
    let height = image.height.max(1);
    rsx! {
        div { class: wrapper_class,
            match loaded.read().as_ref() {
                Some(Ok(data)) => rsx! { img { class: "block max-h-[360px] w-full rounded-lg object-contain", style: "aspect-ratio: {width} / {height};", src: "data:{content_type};base64,{data}", alt: "Изображение из личного сообщения" } },
                Some(Err(error)) => rsx! { div { class: "flex min-h-24 items-center justify-center rounded-lg bg-red-950/20 p-3 text-center text-xs text-red-200", "{error}" } },
                None => rsx! { div { class: "flex min-h-24 items-center justify-center rounded-lg bg-zinc-900/60", span { class: "h-5 w-5 animate-spin rounded-full border-2 border-zinc-700 border-t-blue-400" } } },
            }
        }
    }
}
