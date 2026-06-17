//! Вид вложения-изображения в текстовом чате.

use cheenhub_contracts::realtime::TextChatImageAttachment;
use dioxus::html::geometry::WheelDelta;
use dioxus::prelude::*;

use crate::features::realtime::RealtimeHandle;

use super::realtime;

/// Рендерит одно вложение-изображение текстового чата, загруженное через realtime.
#[component]
pub(super) fn ChatImageAttachment(attachment: TextChatImageAttachment) -> Element {
    let realtime = use_context::<RealtimeHandle>();
    let attachment_id = attachment.id.clone();
    let content_type = attachment.content_type.clone();
    let mut viewer_open = use_signal(|| false);
    let mut zoom = use_signal(|| 1.0_f64);
    let mut pan_x = use_signal(|| 0.0_f64);
    let mut pan_y = use_signal(|| 0.0_f64);
    let mut drag_origin = use_signal(|| None::<(f64, f64, f64, f64)>);
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
        async move { realtime::load_chat_image(&realtime, attachment_id).await }
    });
    let zoom_percent = (zoom() * 100.0).round() as i32;
    let viewer_image_class = if drag_origin().is_some() {
        "chat-image-viewer-image block cursor-grabbing select-none rounded-[10px] object-contain shadow-[0_24px_90px_rgba(0,0,0,0.65)] will-change-transform"
    } else {
        "chat-image-viewer-image block cursor-grab select-none rounded-[10px] object-contain shadow-[0_24px_90px_rgba(0,0,0,0.65)] will-change-transform"
    };

    rsx! {
        div {
            class: "mt-2 inline-block max-w-full overflow-hidden rounded-[14px] border border-zinc-700/80 bg-zinc-950/70 shadow-[0_0_0_1px_rgba(255,255,255,0.035),0_12px_32px_rgba(0,0,0,0.28)]",
            style: "width: min({preview_width}px, 100%);",
            match image.read().as_ref() {
                Some(Ok(loaded)) => rsx! {
                    button {
                        r#type: "button",
                        class: "group block w-full cursor-zoom-in bg-zinc-950/80 p-1 transition-colors hover:bg-zinc-900/80",
                        "aria-label": "Открыть изображение",
                        onclick: move |_| {
                            zoom.set(1.0);
                            pan_x.set(0.0);
                            pan_y.set(0.0);
                            drag_origin.set(None);
                            viewer_open.set(true);
                        },
                        img {
                            class: "block w-full rounded-[10px] object-contain transition-opacity group-hover:opacity-95",
                            style: "aspect-ratio: {aspect_width} / {aspect_height};",
                            src: "data:{loaded.content_type};base64,{loaded.data_base64}",
                            alt: "Изображение из сообщения",
                        }
                    }
                },
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
        div { class: "sr-only",
            "{content_type}"
        }
        if viewer_open() {
            if let Some(Ok(loaded)) = image.read().as_ref() {
                div {
                    class: "chat-image-viewer flex flex-col bg-black text-zinc-100 backdrop-blur-sm",
                    onwheel: move |event| {
                        event.prevent_default();
                        let delta_y = match event.delta() {
                            WheelDelta::Pixels(delta) => delta.y,
                            WheelDelta::Lines(delta) => delta.y * 24.0,
                            WheelDelta::Pages(delta) => delta.y * 240.0,
                        };
                        let factor = if delta_y < 0.0 { 1.12 } else { 0.89 };
                        zoom.set((zoom() * factor).clamp(0.35, 5.0));
                    },
                    onmousemove: move |event| {
                        if let Some((start_x, start_y, origin_x, origin_y)) = drag_origin() {
                            let point = event.client_coordinates();
                            pan_x.set(origin_x + point.x - start_x);
                            pan_y.set(origin_y + point.y - start_y);
                        }
                    },
                    onmouseup: move |_| drag_origin.set(None),
                    onmouseleave: move |_| drag_origin.set(None),
                    div { class: "flex h-14 shrink-0 items-center justify-between gap-3 border-b border-white/10 bg-zinc-950/80 px-4 backdrop-blur-xl",
                        div { class: "min-w-0 text-[12px] font-medium text-zinc-300",
                            "{attachment.width}×{attachment.height} · {zoom_percent}%"
                        }
                        div { class: "flex items-center gap-2",
                            button {
                                r#type: "button",
                                class: "flex h-9 w-9 items-center justify-center rounded-xl border border-white/10 bg-white/5 text-zinc-200 transition hover:border-white/20 hover:bg-white/10",
                                "aria-label": "Уменьшить",
                                onclick: move |event| {
                                    event.stop_propagation();
                                    zoom.set((zoom() * 0.8).clamp(0.35, 5.0));
                                },
                                svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                                    path { stroke_linecap: "round", d: "M5 12h14" }
                                }
                            }
                            button {
                                r#type: "button",
                                class: "flex h-9 w-9 items-center justify-center rounded-xl border border-white/10 bg-white/5 text-zinc-200 transition hover:border-white/20 hover:bg-white/10",
                                "aria-label": "Сбросить масштаб",
                                onclick: move |event| {
                                    event.stop_propagation();
                                    zoom.set(1.0);
                                    pan_x.set(0.0);
                                    pan_y.set(0.0);
                                    drag_origin.set(None);
                                },
                                svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M4 4v6h6M20 20v-6h-6M5 19A9 9 0 0 0 19 5" }
                                }
                            }
                            button {
                                r#type: "button",
                                class: "flex h-9 w-9 items-center justify-center rounded-xl border border-white/10 bg-white/5 text-zinc-200 transition hover:border-white/20 hover:bg-white/10",
                                "aria-label": "Увеличить",
                                onclick: move |event| {
                                    event.stop_propagation();
                                    zoom.set((zoom() * 1.25).clamp(0.35, 5.0));
                                },
                                svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                                    path { stroke_linecap: "round", d: "M12 5v14M5 12h14" }
                                }
                            }
                            button {
                                r#type: "button",
                                class: "flex h-9 w-9 items-center justify-center rounded-xl border border-white/10 bg-white/5 text-zinc-200 transition hover:border-white/20 hover:bg-white/10",
                                "aria-label": "Закрыть",
                                onclick: move |_| {
                                    drag_origin.set(None);
                                    viewer_open.set(false);
                                },
                                svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 6l12 12M18 6 6 18" }
                                }
                            }
                        }
                    }
                    button {
                        r#type: "button",
                        class: "relative min-h-0 flex-1 overflow-auto p-6",
                        "aria-label": "Закрыть просмотр изображения",
                        onclick: move |_| {
                            drag_origin.set(None);
                            viewer_open.set(false);
                        },
                        div { class: "flex min-h-full min-w-full items-center justify-center",
                            img {
                                class: viewer_image_class,
                                style: "transform: translate({pan_x()}px, {pan_y()}px) scale({zoom()}); transform-origin: center center;",
                                src: "data:{loaded.content_type};base64,{loaded.data_base64}",
                                alt: "Изображение из сообщения",
                                onmousedown: move |event| {
                                    event.prevent_default();
                                    event.stop_propagation();
                                    let point = event.client_coordinates();
                                    drag_origin.set(Some((point.x, point.y, pan_x(), pan_y())));
                                },
                                onclick: move |event| event.stop_propagation(),
                            }
                        }
                    }
                }
            }
        }
    }
}
