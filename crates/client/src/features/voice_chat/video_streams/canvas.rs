//! Плитка одного видеопотока: платформенный canvas либо штатный HTML video.

use std::{cell::RefCell, rc::Rc};

use dioxus::prelude::*;
use futures_util::StreamExt;
use uuid::Uuid;

use super::{ParticipantVideoHandle, ParticipantVideoSource, backend::ParticipantVideoRenderer};

/// Показывает один источник участника и сообщает родителю размеры изображения.
#[component]
pub(crate) fn ParticipantVideoCanvas(
    user_id: String,
    source: ParticipantVideoSource,
    on_dimensions: Option<EventHandler<(u32, u32)>>,
) -> Element {
    let video = use_context::<ParticipantVideoHandle>();
    let target_id = use_hook(move || format!("{}-{}", source.id_prefix(), Uuid::new_v4().simple()));
    let mut waiting = use_signal(|| true);
    let mut playback_url = use_signal(|| None::<String>);
    let mut failed = use_signal(|| false);
    let renderer_owner =
        use_hook(|| Rc::new(RefCell::new(None::<Rc<dyn ParticipantVideoRenderer>>)));
    use_drop({
        let owner = renderer_owner.clone();
        move || {
            if let Some(renderer) = owner.borrow_mut().take() {
                renderer.close();
            }
        }
    });

    use_hook({
        let target_id = target_id.clone();
        let user_id = user_id.clone();
        let owner = renderer_owner.clone();
        move || {
            spawn(async move {
                let mut frames = video.subscribe_stream(source, user_id.clone());
                let on_dimensions = Rc::new(move |width, height| {
                    if let Some(handler) = on_dimensions {
                        handler.call((width, height));
                    }
                });
                let renderer = match video.create_renderer(
                    target_id,
                    user_id.clone(),
                    source,
                    on_dimensions,
                ) {
                    Ok(renderer) => renderer,
                    Err(error) => {
                        warn!(%error, %user_id, source = source.label(), "failed to create participant video renderer");
                        failed.set(true);
                        return;
                    }
                };
                let url = renderer.playback_url();
                let native_player = url.is_some();
                playback_url.set(url);
                *owner.borrow_mut() = Some(renderer.clone());
                while let Some(frame) = frames.next().await {
                    if *failed.peek() {
                        break;
                    }
                    if frame.key_frame && !native_player {
                        waiting.set(false);
                    }
                    let sequence = frame.sequence;
                    if let Err(error) = renderer.decode(frame).await {
                        warn!(%error, %user_id, sequence, source = source.label(), "failed to render participant video frame");
                        if native_player {
                            failed.set(true);
                            break;
                        }
                    }
                }
                renderer.close();
            })
        }
    });

    rsx! {
        div {
            id: "{target_id}",
            class: "absolute inset-0 z-0 h-full w-full overflow-hidden bg-zinc-950",
            if let Some(url) = playback_url() {
                video {
                    src: url,
                    autoplay: true,
                    muted: true,
                    playsinline: true,
                    class: "absolute inset-0 h-full w-full object-contain",
                    "aria-hidden": "true",
                    onplaying: {
                        let user_id = user_id.clone();
                        move |_| {
                            if *waiting.peek() {
                                info!(%user_id, source = source.label(), "participant video playback started");
                                waiting.set(false);
                            }
                        }
                    },
                    onerror: {
                        let user_id = user_id.clone();
                        let owner = renderer_owner.clone();
                        move |_| {
                            if !*failed.peek() {
                                warn!(%user_id, source = source.label(), "participant video media player failed");
                                failed.set(true);
                                if let Some(renderer) = owner.borrow().as_ref() {
                                    renderer.close();
                                }
                            }
                        }
                    },
                }
            }
            if failed() {
                div {
                    role: "status",
                    class: "absolute inset-0 z-10 flex items-center justify-center bg-zinc-950/90 p-5 text-center text-sm text-zinc-300",
                    "Не удалось показать видео. Переподключитесь к звонку, чтобы повторить попытку."
                }
            } else if waiting() {
                div {
                    role: "status",
                    class: "pointer-events-none absolute inset-0 z-10 flex items-center justify-center bg-zinc-950/70",
                    span { class: "sr-only", "Загружаем видео" }
                    div {
                        class: "h-8 w-8 animate-spin rounded-full border-2 border-zinc-700 border-t-sky-300",
                        "aria-hidden": "true",
                    }
                }
            }
        }
    }
}
