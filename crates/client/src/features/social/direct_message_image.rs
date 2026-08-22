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
    let is_own = author_user_id == current_user.id;
    let wrapper_class = if is_own {
        "inline-block shrink-0 overflow-hidden rounded-[14px] border border-blue-500/20 bg-blue-950/20 shadow-[0_0_0_1px_rgba(255,255,255,0.035),0_12px_32px_rgba(0,0,0,0.28)]"
    } else {
        "inline-block shrink-0 overflow-hidden rounded-[14px] border border-zinc-700/80 bg-zinc-950/70 shadow-[0_0_0_1px_rgba(255,255,255,0.035),0_12px_32px_rgba(0,0,0,0.28)]"
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
    let preview = preview_geometry(image.width, image.height);
    let shell_styles = image_shell_styles(preview);
    let image_state = loaded.read();
    rsx! {
        div { class: wrapper_class, style: "{shell_styles.wrapper}",
            match image_state.as_ref() {
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
                            class: "group block w-full cursor-zoom-in bg-zinc-950/80 p-1 transition-colors hover:bg-zinc-900/80",
                            "aria-label": "Открыть изображение из личного сообщения",
                            onclick: move |_| {
                                let image = viewer_image.clone();
                                let element = thumbnail_element();
                                viewer.open(image, element);
                            },
                            img {
                                onmounted: move |event| thumbnail_element.set(Some(event.data.clone())),
                                class: "block w-full rounded-[10px] object-contain transition-opacity group-hover:opacity-95",
                                style: "{shell_styles.surface}",
                                src: "data:{content_type};base64,{data}",
                                alt: "Изображение из личного сообщения",
                            }
                        }
                    }
                }
                Some(Err(error)) => rsx! { div { class: "w-full bg-zinc-950/80 p-1", div { class: "flex w-full items-center justify-center rounded-[10px] bg-red-950/20 px-3 py-2 text-center text-[12px] text-red-200", role: "alert", "aria-live": "assertive", style: "{shell_styles.surface}", p { class: "min-w-0 break-words", "{error}" } } } },
                None => rsx! { div { class: "w-full bg-zinc-950/80 p-1", div { class: "relative flex w-full items-center justify-center overflow-hidden rounded-[10px] bg-zinc-900/45", role: "status", "aria-label": "Загрузка изображения", style: "{shell_styles.surface}", div { class: "pointer-events-none absolute inset-0 -translate-x-full animate-[chat-image-shimmer_1.35s_ease-in-out_infinite] bg-gradient-to-r from-transparent via-white/10 to-transparent" }, div { class: "pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_center,rgba(255,255,255,0.08),transparent_42%)]" }, span { class: "relative z-10 h-5 w-5 animate-spin rounded-full border-2 border-zinc-700 border-t-blue-400", "aria-hidden": "true" } } } },
            }
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::{
        IMAGE_PREVIEW_MAX_HEIGHT, IMAGE_PREVIEW_MAX_WIDTH, PreviewGeometry, image_shell_styles,
        preview_geometry,
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
}
