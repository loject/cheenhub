//! Полноэкранный просмотр изображений чата.

use std::rc::Rc;

use dioxus::html::geometry::WheelDelta;
use dioxus::logger::tracing::info;
use dioxus::prelude::*;

mod geometry;

#[cfg(test)]
use geometry::transformed_point;
use geometry::{FlipTransform, flip_transform, is_valid_geometry};
/// Данные уже загруженного изображения для полноэкранного просмотра чата.
#[derive(Clone)]
pub(crate) struct ChatImageViewerImage {
    /// Стабильный идентификатор вложения для диагностики.
    pub(crate) attachment_id: String,
    /// Проверенный MIME-тип изображения.
    pub(crate) content_type: String,
    /// Содержимое изображения в Base64.
    pub(crate) data_base64: String,
    /// Ширина изображения в пикселях.
    pub(crate) width: i32,
    /// Высота изображения в пикселях.
    pub(crate) height: i32,
}

/// Снимок геометрии thumbnail относительно viewport.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ChatImageViewerGeometry {
    /// Координата левой границы в пикселях.
    pub(crate) x: f64,
    /// Координата верхней границы в пикселях.
    pub(crate) y: f64,
    /// Ширина в пикселях.
    pub(crate) width: f64,
    /// Высота в пикселях.
    pub(crate) height: f64,
}

/// Источник shared-element перехода изображения чата.
#[derive(Clone)]
pub(crate) struct ChatImageViewerSource {
    element: Option<Rc<MountedData>>,
    geometry: Option<ChatImageViewerGeometry>,
}

/// Измеряет thumbnail непосредственно перед открытием просмотра.
pub(crate) async fn measure_chat_image_source(
    element: Option<Rc<MountedData>>,
) -> ChatImageViewerSource {
    let geometry = match element.as_ref() {
        Some(element) => element.get_client_rect().await.ok().and_then(|rect| {
            let geometry = ChatImageViewerGeometry {
                x: rect.origin.x,
                y: rect.origin.y,
                width: rect.size.width,
                height: rect.size.height,
            };
            is_valid_geometry(geometry).then_some(geometry)
        }),
        None => None,
    };

    ChatImageViewerSource { element, geometry }
}

/// Доступ к единственному полноэкранному просмотру изображений чата.
#[derive(Clone, Copy)]
pub(crate) struct ChatImageViewerContext {
    state: Signal<ViewerState>,
    open_request: Signal<u64>,
    zoom: Signal<f64>,
    pan_x: Signal<f64>,
    pan_y: Signal<f64>,
    drag_origin: Signal<Option<(f64, f64, f64, f64)>>,
}

impl ChatImageViewerContext {
    /// Регистрирует открытие и измеряет thumbnail после синхронной защиты от устаревшего запроса.
    pub(crate) fn open(self, image: ChatImageViewerImage, element: Option<Rc<MountedData>>) {
        let mut open_request = self.open_request;
        let request_generation = open_request.with_mut(|generation| {
            *generation = generation.saturating_add(1);
            *generation
        });
        spawn(async move {
            let source = measure_chat_image_source(element).await;
            if !is_current_open_request((self.open_request)(), request_generation) {
                return;
            }

            info!(attachment_id = %image.attachment_id, "opening chat image viewer");
            let mut state = self.state;
            state.with_mut(|state| open_viewer(state, image, source));
        });
    }

    /// Запускает закрытие просмотра после повторного измерения thumbnail.
    pub(crate) fn close(self, attachment_id: String) {
        let current_state = (self.state)();
        if !matches_active_viewer(&current_state, current_state.generation, &attachment_id) {
            return;
        }

        let generation = current_state.generation;
        let source = current_state.source;
        spawn(async move {
            let source = measure_chat_image_source(source.element).await;
            let mut state = self.state;
            if state
                .with_mut(|state| {
                    request_viewer_close(state, generation, &attachment_id, source.geometry)
                })
                .is_some()
            {
                info!(attachment_id, "closing chat image viewer");
                let mut drag_origin = self.drag_origin;
                let mut zoom = self.zoom;
                let mut pan_x = self.pan_x;
                let mut pan_y = self.pan_y;
                drag_origin.set(None);
                zoom.set(1.0);
                pan_x.set(0.0);
                pan_y.set(0.0);
            }
        });
    }

    fn finish_close(self, closing_generation: u64) {
        let mut state = self.state;
        state.with_mut(|state| finish_viewer_close(state, closing_generation));
    }
}

/*
 * Состояние открытия меняется только после измерения, а счётчик запросов в
 * контексте исключает отображение результата более старого измерения.
 */
#[derive(Clone)]
struct ViewerState {
    image: Option<ChatImageViewerImage>,
    source: ChatImageViewerSource,
    generation: u64,
    closing_generation: Option<u64>,
    target_geometry: Option<ChatImageViewerGeometry>,
    transition: SharedTransition,
}

#[derive(Clone, Copy, PartialEq)]
enum SharedTransition {
    Pending,
    Enter(Option<FlipTransform>),
    Exit(Option<FlipTransform>),
}

fn open_viewer(
    state: &mut ViewerState,
    image: ChatImageViewerImage,
    source: ChatImageViewerSource,
) {
    state.generation = state.generation.saturating_add(1);
    state.image = Some(image);
    state.source = source;
    state.closing_generation = None;
    state.target_geometry = None;
    state.transition = SharedTransition::Pending;
}

fn matches_active_viewer(state: &ViewerState, generation: u64, attachment_id: &str) -> bool {
    state.generation == generation
        && state.closing_generation.is_none()
        && state
            .image
            .as_ref()
            .is_some_and(|image| image.attachment_id == attachment_id)
}

fn is_current_open_request(current_generation: u64, request_generation: u64) -> bool {
    current_generation == request_generation
}

fn start_viewer_enter(
    state: &mut ViewerState,
    generation: u64,
    target: Option<ChatImageViewerGeometry>,
) {
    if state.generation != generation || state.closing_generation.is_some() {
        return;
    }

    state.target_geometry = target;
    state.transition = SharedTransition::Enter(
        state
            .source
            .geometry
            .zip(target)
            .and_then(|(source, target)| flip_transform(source, target)),
    );
}

fn request_viewer_close(
    state: &mut ViewerState,
    generation: u64,
    attachment_id: &str,
    source: Option<ChatImageViewerGeometry>,
) -> Option<u64> {
    if !matches_active_viewer(state, generation, attachment_id) {
        return None;
    }

    state.closing_generation = Some(state.generation);
    state.transition = SharedTransition::Exit(
        state
            .target_geometry
            .zip(source)
            .and_then(|(target, source)| flip_transform(source, target)),
    );
    state.closing_generation
}

fn finish_viewer_close(state: &mut ViewerState, closing_generation: u64) {
    if state.closing_generation == Some(closing_generation)
        && state.generation == closing_generation
    {
        state.image = None;
        state.closing_generation = None;
        state.target_geometry = None;
        state.transition = SharedTransition::Pending;
    }
}

/// Размещает полноэкранный просмотр рядом с оболочкой приложения, вне обрезаемых поверхностей чата.
#[component]
pub(crate) fn ChatImageViewerProvider(children: Element) -> Element {
    let state = use_signal(|| ViewerState {
        image: None,
        source: ChatImageViewerSource {
            element: None,
            geometry: None,
        },
        generation: 0,
        closing_generation: None,
        target_geometry: None,
        transition: SharedTransition::Pending,
    });
    let open_request = use_signal(|| 0_u64);
    let mut zoom = use_signal(|| 1.0_f64);
    let mut pan_x = use_signal(|| 0.0_f64);
    let mut pan_y = use_signal(|| 0.0_f64);
    let mut drag_origin = use_signal(|| None::<(f64, f64, f64, f64)>);
    let viewer = ChatImageViewerContext {
        state,
        open_request,
        zoom,
        pan_x,
        pan_y,
        drag_origin,
    };
    use_context_provider(move || viewer);

    rsx! {
        {children}
        if let Some(viewer_image) = state().image {
            {
                let viewer_state = state();
                let closing_generation = viewer_state.closing_generation;
                let generation = viewer_state.generation;
                let shared_class = shared_transition_class(viewer_state.transition);
                let shared_style = shared_transition_style(viewer_state.transition);
                let close_button_attachment_id = viewer_image.attachment_id.clone();
                let close_surface_attachment_id = viewer_image.attachment_id.clone();
                let zoom_percent = (zoom() * 100.0).round() as i32;
                let viewer_image_class = if drag_origin().is_some() {
                    "chat-image-viewer-image block cursor-grabbing select-none rounded-[10px] object-contain shadow-[0_24px_90px_rgba(0,0,0,0.65)] will-change-transform"
                } else {
                    "chat-image-viewer-image block cursor-grab select-none rounded-[10px] object-contain shadow-[0_24px_90px_rgba(0,0,0,0.65)] will-change-transform"
                };
                rsx! {
                    div {
                        class: "chat-image-viewer flex flex-col text-zinc-100",
                        "data-closing": if closing_generation.is_some() { "true" } else { "false" },
                        onwheel: move |event| {
                            event.prevent_default();
                            let delta_y = match event.delta() {
                                WheelDelta::Pixels(delta) => delta.y,
                                WheelDelta::Lines(delta) => delta.y * 24.0,
                                WheelDelta::Pages(delta) => delta.y * 240.0,
                            };
                            zoom.set((zoom() * if delta_y < 0.0 { 1.12 } else { 0.89 }).clamp(0.35, 5.0));
                        },
                        onpointermove: move |event| {
                            if let Some((start_x, start_y, origin_x, origin_y)) = drag_origin() {
                                let point = event.client_coordinates();
                                let (next_pan_x, next_pan_y) = panned_coordinates((start_x, start_y, origin_x, origin_y), (point.x, point.y));
                                pan_x.set(next_pan_x);
                                pan_y.set(next_pan_y);
                            }
                        },
                        onpointerup: move |_| drag_origin.set(None),
                        onpointercancel: move |_| drag_origin.set(None),
                        onpointerleave: move |_| drag_origin.set(None),
                        button { r#type: "button", class: "chat-image-viewer-close flex h-10 w-10 items-center justify-center rounded-xl border border-white/10 bg-zinc-950/75 text-zinc-200 shadow-lg backdrop-blur-xl transition hover:border-white/20 hover:bg-zinc-900/90", "aria-label": "Закрыть просмотр изображения", title: "Закрыть", onclick: move |_| viewer.close(close_button_attachment_id.clone()), svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 6l12 12M18 6 6 18" } } }
                        div { class: "chat-image-viewer-controls flex items-center justify-end gap-2",
                            div { class: "hidden min-w-0 text-right text-[12px] font-medium text-zinc-300 sm:block", "{viewer_image.width}×{viewer_image.height} · {zoom_percent}%" }
                            div { class: "flex items-center gap-2",
                                button { r#type: "button", class: "flex h-9 w-9 items-center justify-center rounded-xl border border-white/10 bg-white/5 text-zinc-200 transition hover:border-white/20 hover:bg-white/10", "aria-label": "Уменьшить", onclick: move |event| { event.stop_propagation(); zoom.set((zoom() * 0.8).clamp(0.35, 5.0)); }, svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", path { stroke_linecap: "round", d: "M5 12h14" } } }
                                button { r#type: "button", class: "flex h-9 min-w-9 items-center justify-center rounded-xl border border-white/10 bg-white/5 px-1.5 text-[11px] font-bold tabular-nums text-zinc-200 transition hover:border-white/20 hover:bg-white/10", "aria-label": "Сбросить масштаб", title: "Сбросить масштаб", onclick: move |event| { event.stop_propagation(); zoom.set(1.0); pan_x.set(0.0); pan_y.set(0.0); drag_origin.set(None); }, "1:1" }
                                button { r#type: "button", class: "flex h-9 w-9 items-center justify-center rounded-xl border border-white/10 bg-white/5 text-zinc-200 transition hover:border-white/20 hover:bg-white/10", "aria-label": "Увеличить", onclick: move |event| { event.stop_propagation(); zoom.set((zoom() * 1.25).clamp(0.35, 5.0)); }, svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 5v14M5 12h14" } } }
                            }
                        }
                        button { r#type: "button", class: "chat-image-viewer-surface relative min-h-0 flex-1 overflow-auto p-6", "aria-label": "Закрыть просмотр изображения", onclick: move |_| viewer.close(close_surface_attachment_id.clone()),
                            div { class: "flex min-h-full min-w-full items-center justify-center",
                                div { class: "chat-image-viewer-shared inline-block {shared_class}", style: "{shared_style}", onmounted: move |event| {
                                    let element = event.data.clone();
                                    spawn(async move {
                                        let target = measure_chat_image_source(Some(element)).await.geometry;
                                        let mut state = state;
                                        state.with_mut(|state| start_viewer_enter(state, generation, target));
                                    });
                                },
                                    img { class: "{viewer_image_class} touch-none", style: "transform: translate({pan_x()}px, {pan_y()}px) scale({zoom()}); transform-origin: center center;", src: "data:{viewer_image.content_type};base64,{viewer_image.data_base64}", alt: "Изображение из сообщения", onpointerdown: move |event| { event.prevent_default(); event.stop_propagation(); let point = event.client_coordinates(); drag_origin.set(Some((point.x, point.y, pan_x(), pan_y()))); }, onclick: move |event| event.stop_propagation() }
                                }
                            }
                        }
                        if let Some(closing_generation) = closing_generation {
                            div { class: "chat-image-viewer-exit-sentinel", "aria-hidden": "true", onanimationend: move |_| viewer.finish_close(closing_generation) }
                        }
                    }
                }
            }
        }
    }
}

fn shared_transition_class(transition: SharedTransition) -> &'static str {
    match transition {
        SharedTransition::Pending => "chat-image-viewer-shared-pending",
        SharedTransition::Enter(Some(_)) => "chat-image-viewer-shared-enter",
        SharedTransition::Enter(None) => "chat-image-viewer-shared-fallback-enter",
        SharedTransition::Exit(Some(_)) => "chat-image-viewer-shared-exit",
        SharedTransition::Exit(None) => "chat-image-viewer-shared-fallback-exit",
    }
}

fn shared_transition_style(transition: SharedTransition) -> String {
    let transform = match transition {
        SharedTransition::Enter(Some(transform)) | SharedTransition::Exit(Some(transform)) => {
            transform
        }
        _ => return String::new(),
    };
    format!(
        "--flip-x: {}px; --flip-y: {}px; --flip-scale: {};",
        transform.x, transform.y, transform.scale
    )
}

/// Вычисляет смещение изображения относительно исходной точки перетаскивания.
fn panned_coordinates(
    (start_x, start_y, origin_x, origin_y): (f64, f64, f64, f64),
    (pointer_x, pointer_y): (f64, f64),
) -> (f64, f64) {
    (
        origin_x + pointer_x - start_x,
        origin_y + pointer_y - start_y,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_source_geometry() {
        assert!(!is_valid_geometry(ChatImageViewerGeometry {
            x: f64::NAN,
            y: 0.0,
            width: 20.0,
            height: 20.0
        }));
        assert!(!is_valid_geometry(ChatImageViewerGeometry {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 20.0
        }));
    }

    #[test]
    fn uniform_flip_maps_top_left_and_opposite_corners_to_source() {
        let source = geometry(10.0, 20.0, 100.0, 50.0);
        let target = geometry(50.0, 70.0, 200.0, 100.0);
        let transform = flip_transform(source, target).expect("valid geometry");

        assert_eq!(transformed_point(target, transform, 0.0, 0.0), (10.0, 20.0));
        assert_eq!(
            transformed_point(target, transform, target.width, target.height),
            (110.0, 70.0)
        );
    }

    #[test]
    fn uniform_flip_keeps_centers_aligned_when_rect_ratios_differ() {
        let source = geometry(20.0, 30.0, 120.0, 60.0);
        let target = geometry(100.0, 100.0, 300.0, 100.0);
        let transform = flip_transform(source, target).expect("valid geometry");

        assert_eq!(transform.scale, 0.4);
        assert_eq!(
            transformed_point(target, transform, target.width / 2.0, target.height / 2.0),
            (80.0, 60.0)
        );
    }

    #[test]
    fn stale_exit_completion_does_not_close_a_reopened_viewer() {
        let mut state = test_state("first", 7);
        let closing_generation =
            request_viewer_close(&mut state, 7, "first", Some(geometry(1.0, 1.0, 10.0, 10.0)));
        open_viewer(&mut state, test_image("second"), empty_source());
        finish_viewer_close(&mut state, closing_generation.expect("close starts"));
        assert_eq!(
            state.image.expect("viewer remains open").attachment_id,
            "second"
        );
    }

    #[test]
    fn stale_open_request_is_rejected_before_it_can_replace_newer_thumbnail() {
        assert!(!is_current_open_request(8, 7));
        assert!(is_current_open_request(8, 8));
    }

    #[test]
    fn stale_close_request_cannot_close_a_reopened_viewer() {
        let mut state = test_state("second", 8);
        assert_eq!(
            request_viewer_close(&mut state, 7, "first", Some(geometry(1.0, 1.0, 10.0, 10.0))),
            None
        );
    }

    #[test]
    fn pointer_pan_preserves_existing_offset() {
        assert_eq!(
            panned_coordinates((120.0, 80.0, 16.0, -24.0), (155.0, 40.0)),
            (51.0, -64.0)
        );
    }

    fn geometry(x: f64, y: f64, width: f64, height: f64) -> ChatImageViewerGeometry {
        ChatImageViewerGeometry {
            x,
            y,
            width,
            height,
        }
    }
    fn empty_source() -> ChatImageViewerSource {
        ChatImageViewerSource {
            element: None,
            geometry: None,
        }
    }
    fn test_image(attachment_id: &str) -> ChatImageViewerImage {
        ChatImageViewerImage {
            attachment_id: attachment_id.to_owned(),
            content_type: "image/png".to_owned(),
            data_base64: String::new(),
            width: 1,
            height: 1,
        }
    }
    fn test_state(attachment_id: &str, generation: u64) -> ViewerState {
        ViewerState {
            image: Some(test_image(attachment_id)),
            source: empty_source(),
            generation,
            closing_generation: None,
            target_geometry: Some(geometry(0.0, 0.0, 1.0, 1.0)),
            transition: SharedTransition::Enter(None),
        }
    }
}
