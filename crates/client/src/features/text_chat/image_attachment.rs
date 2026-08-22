//! Вид вложения-изображения в текстовом чате.

use std::rc::Rc;

use cheenhub_contracts::realtime::TextChatImageAttachment;
use dioxus::logger::tracing::warn;
use dioxus::prelude::*;
use futures_util::FutureExt;
use futures_util::future::{Either, select};
use uuid::Uuid;

use crate::features::app::components::chat_image_viewer::{
    ChatImageViewerContext, ChatImageViewerImage,
};
use crate::features::realtime::RealtimeHandle;
use crate::features::runtime::sleep_duration;

use super::realtime;

/// Рендерит одно вложение-изображение текстового чата, загруженное через realtime.
#[component]
pub(super) fn ChatImageAttachment(
    server_id: String,
    room_id: String,
    attachment: TextChatImageAttachment,
    is_own: bool,
) -> Element {
    let realtime = use_context::<RealtimeHandle>();
    let load_key = ChatImageLoadKey::parse(&server_id, &room_id, &attachment.id);
    let viewer = use_context::<ChatImageViewerContext>();
    let mut thumbnail_element = use_signal(|| None::<Rc<MountedData>>);
    let mut reload_generation = use_signal(|| 0_u64);
    let content_type = attachment.content_type.clone();
    let preview = preview_geometry(attachment.width, attachment.height);
    let shell_styles = image_shell_styles(preview);
    let wrapper_class = if is_own {
        "inline-block shrink-0 overflow-hidden rounded-[14px] border border-blue-500/20 bg-blue-950/20 shadow-[0_0_0_1px_rgba(255,255,255,0.035),0_12px_32px_rgba(0,0,0,0.28)]"
    } else {
        "inline-block shrink-0 overflow-hidden rounded-[14px] border border-zinc-700/80 bg-zinc-950/70 shadow-[0_0_0_1px_rgba(255,255,255,0.035),0_12px_32px_rgba(0,0,0,0.28)]"
    };
    let load_server_id = server_id.clone();
    let load_room_id = room_id.clone();
    let load_attachment_id = attachment.id.clone();
    let image = use_resource(move || {
        let realtime = realtime.clone();
        let load_key = load_key.clone();
        let server_id = load_server_id.clone();
        let room_id = load_room_id.clone();
        let attachment_id = load_attachment_id.clone();
        let generation = reload_generation();
        async move {
            let result = match load_key {
                Ok(load_key) => load_chat_image_with_timeout(&realtime, load_key, generation).await,
                Err(error) => Err(error),
            };
            if let Err(error) = &result {
                warn!(
                    server_id = %server_id,
                    room_id = %room_id,
                    attachment_id = %attachment_id,
                    generation,
                    %error,
                    "failed to load text chat image attachment"
                );
            }
            result
        }
    });

    let image_state = image.read();

    rsx! {
        div {
            class: wrapper_class,
            style: "{shell_styles.wrapper}",
            match image_state.as_ref() {
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
                                style: "{shell_styles.surface}",
                                src: "data:{loaded.content_type};base64,{loaded.data_base64}",
                                alt: "Изображение из сообщения",
                            }
                        }
                    }
                }
                Some(Err(error)) => rsx! {
                    div { class: "w-full bg-zinc-950/80 p-1",
                        div {
                            class: "flex w-full flex-col items-center justify-center gap-2 rounded-[10px] bg-red-950/20 px-3 py-2 text-center text-[12px] text-red-200",
                            style: "{shell_styles.surface}",
                            p { class: "min-w-0 break-words", "{error}" }
                            button {
                                r#type: "button",
                                class: "inline-flex w-fit min-w-0 max-w-full items-center justify-center rounded-lg border border-red-300/25 px-2.5 py-1 text-center text-[11px] font-medium leading-4 whitespace-normal break-words text-red-100 transition-colors hover:border-red-200/45 hover:bg-red-400/10 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-red-100 focus-visible:ring-offset-2 focus-visible:ring-offset-red-950/20",
                                onclick: move |_| {
                                    let next_generation = reload_generation().saturating_add(1);
                                    info!(
                                        server_id = %server_id,
                                        room_id = %room_id,
                                        attachment_id = %attachment.id,
                                        generation = next_generation,
                                        "retrying text chat image attachment load"
                                    );
                                    reload_generation.set(next_generation);
                                },
                                "Повторить"
                            }
                        }
                    }
                },
                None => rsx! {
                    div { class: "w-full bg-zinc-950/80 p-1",
                        div {
                            class: "relative flex w-full items-center justify-center overflow-hidden rounded-[10px] bg-zinc-900/45",
                            role: "status",
                            "aria-label": "Загрузка изображения",
                            style: "{shell_styles.surface}",
                            div { class: "pointer-events-none absolute inset-0 -translate-x-full animate-[chat-image-shimmer_1.35s_ease-in-out_infinite] bg-gradient-to-r from-transparent via-white/10 to-transparent" }
                            div { class: "pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_center,rgba(255,255,255,0.08),transparent_42%)]" }
                            span { class: "relative z-10 h-5 w-5 animate-spin rounded-full border-2 border-zinc-700 border-t-blue-400", "aria-hidden": "true" }
                        }
                    }
                },
            }
        }
        div { class: "sr-only", "{content_type}" }
    }
}

const IMAGE_LOAD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(12);
const IMAGE_PREVIEW_MAX_WIDTH: f64 = 520.0;
const IMAGE_PREVIEW_MAX_HEIGHT: f64 = 360.0;
const FALLBACK_PREVIEW_WIDTH: f64 = 280.0;
const FALLBACK_PREVIEW_HEIGHT: f64 = 210.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PreviewGeometry {
    width: i32,
    height: i32,
}

fn preview_geometry(width: i32, height: i32) -> PreviewGeometry {
    if width <= 0 || height <= 0 {
        return fallback_geometry();
    }

    let source_width = f64::from(width);
    let source_height = f64::from(height);
    let scale = (IMAGE_PREVIEW_MAX_WIDTH / source_width)
        .min(IMAGE_PREVIEW_MAX_HEIGHT / source_height)
        .min(1.0);

    PreviewGeometry {
        width: (source_width * scale)
            .round()
            .clamp(1.0, IMAGE_PREVIEW_MAX_WIDTH) as i32,
        height: (source_height * scale)
            .round()
            .clamp(1.0, IMAGE_PREVIEW_MAX_HEIGHT) as i32,
    }
}

fn fallback_geometry() -> PreviewGeometry {
    PreviewGeometry {
        width: FALLBACK_PREVIEW_WIDTH as i32,
        height: FALLBACK_PREVIEW_HEIGHT as i32,
    }
}

struct ImageShellStyles {
    wrapper: String,
    surface: String,
}

fn image_shell_styles(preview: PreviewGeometry) -> ImageShellStyles {
    ImageShellStyles {
        wrapper: format!("width: {}px; max-width: 100%;", preview.width),
        surface: format!("aspect-ratio: {} / {};", preview.width, preview.height),
    }
}

/// Ключ загрузки связывает ресурс с конкретной комнатой и вложением, а не только с позицией DOM.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ChatImageLoadKey {
    server_id: Uuid,
    room_id: Uuid,
    attachment_id: Uuid,
}

impl ChatImageLoadKey {
    fn parse(server_id: &str, room_id: &str, attachment_id: &str) -> Result<Self, String> {
        Ok(Self {
            server_id: Uuid::parse_str(server_id)
                .map_err(|_| "Не удалось определить сервер изображения.".to_owned())?,
            room_id: Uuid::parse_str(room_id)
                .map_err(|_| "Не удалось определить комнату изображения.".to_owned())?,
            attachment_id: Uuid::parse_str(attachment_id)
                .map_err(|_| "Не удалось определить вложение изображения.".to_owned())?,
        })
    }

    /// Преобразует typed UUID-ключ в строку только на границе ключа Dioxus VNode.
    pub(super) fn render_key(server_id: &str, room_id: &str, attachment_id: &str) -> String {
        match Self::parse(server_id, room_id, attachment_id) {
            Ok(key) => format!("{}:{}/{}", key.server_id, key.room_id, key.attachment_id),
            Err(_) => format!("invalid-image:{server_id}:{room_id}:{attachment_id}"),
        }
    }
}

async fn load_chat_image_with_timeout(
    realtime: &RealtimeHandle,
    load_key: ChatImageLoadKey,
    generation: u64,
) -> Result<cheenhub_contracts::realtime::ChatImageLoadedResponse, String> {
    info!(
        server_id = %load_key.server_id,
        room_id = %load_key.room_id,
        attachment_id = %load_key.attachment_id,
        generation,
        "starting text chat image attachment load"
    );
    let request = realtime::load_chat_image(realtime, load_key.attachment_id).boxed_local();
    let timeout = sleep_duration(IMAGE_LOAD_TIMEOUT).boxed_local();

    match select(request, timeout).await {
        Either::Left((Ok(image), _)) => {
            info!(
                server_id = %load_key.server_id,
                room_id = %load_key.room_id,
                attachment_id = %load_key.attachment_id,
                generation,
                "loaded text chat image attachment"
            );
            Ok(image)
        }
        Either::Left((Err(error), _)) => Err(format!(
            "Не удалось загрузить изображение. Попробуй ещё раз. ({error})"
        )),
        Either::Right(((), _)) => {
            Err("Изображение не загрузилось вовремя. Попробуй ещё раз.".to_owned())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ChatImageLoadKey, IMAGE_PREVIEW_MAX_HEIGHT, IMAGE_PREVIEW_MAX_WIDTH, PreviewGeometry,
        image_shell_styles, preview_geometry,
    };

    #[test]
    fn image_preview_geometry_is_bounded() {
        struct Case {
            source: (i32, i32),
            preview: PreviewGeometry,
        }

        let cases = [
            Case {
                source: (0, 0),
                preview: PreviewGeometry {
                    width: 280,
                    height: 210,
                },
            },
            Case {
                source: (1, 1_000_000_000),
                preview: PreviewGeometry {
                    width: 1,
                    height: 360,
                },
            },
            Case {
                source: (1_000_000_000, 1),
                preview: PreviewGeometry {
                    width: 520,
                    height: 1,
                },
            },
            Case {
                source: (520, 360),
                preview: PreviewGeometry {
                    width: 520,
                    height: 360,
                },
            },
            Case {
                source: (900, 1_600),
                preview: PreviewGeometry {
                    width: 203,
                    height: 360,
                },
            },
            Case {
                source: (12, 8),
                preview: PreviewGeometry {
                    width: 12,
                    height: 8,
                },
            },
        ];

        for case in cases {
            let preview = preview_geometry(case.source.0, case.source.1);
            assert_eq!(preview, case.preview);
            assert!(preview.width > 0 && preview.height > 0);
            assert!(f64::from(preview.width) <= IMAGE_PREVIEW_MAX_WIDTH);
            assert!(f64::from(preview.height) <= IMAGE_PREVIEW_MAX_HEIGHT);
        }
    }

    #[test]
    fn shell_styles_keep_a_definite_width_outside_intrinsic_percentage_sizing() {
        let styles = image_shell_styles(preview_geometry(1_600, 900));

        assert_eq!(styles.wrapper, "width: 520px; max-width: 100%;");
        assert!(!styles.wrapper.contains("min(100%"));
        assert_eq!(styles.surface, "aspect-ratio: 520 / 293;");
    }

    #[test]
    fn image_load_key_includes_server_room_and_attachment() {
        let server = "00000000-0000-0000-0000-000000000001";
        let room = "00000000-0000-0000-0000-000000000002";
        let attachment = "00000000-0000-0000-0000-000000000003";

        let key = ChatImageLoadKey::parse(server, room, attachment).expect("valid UUID key");
        let another_room =
            ChatImageLoadKey::parse(server, "00000000-0000-0000-0000-000000000004", attachment)
                .expect("valid UUID key");
        let another_server =
            ChatImageLoadKey::parse("00000000-0000-0000-0000-000000000005", room, attachment)
                .expect("valid UUID key");
        let another_attachment =
            ChatImageLoadKey::parse(server, room, "00000000-0000-0000-0000-000000000006")
                .expect("valid UUID key");

        assert_ne!(key, another_room);
        assert_ne!(key, another_server);
        assert_ne!(key, another_attachment);
        assert_ne!(
            ChatImageLoadKey::render_key(server, room, attachment),
            ChatImageLoadKey::render_key(server, room, "00000000-0000-0000-0000-000000000006")
        );
        assert_ne!(
            ChatImageLoadKey::render_key(server, room, attachment),
            ChatImageLoadKey::render_key("00000000-0000-0000-0000-000000000005", room, attachment)
        );
        assert_ne!(
            ChatImageLoadKey::render_key(server, room, attachment),
            ChatImageLoadKey::render_key(
                server,
                "00000000-0000-0000-0000-000000000004",
                attachment
            )
        );
    }
}
