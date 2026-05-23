//! Voice room screen sharing video rendering.

mod backend;
#[cfg(not(target_arch = "wasm32"))]
mod unsupported;
#[cfg(target_arch = "wasm32")]
mod web;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use dioxus::prelude::*;
use futures_channel::mpsc;
use futures_util::StreamExt;
use gloo_timers::future::TimeoutFuture;
use uuid::Uuid;

use self::backend::{ScreenVideoBackend, ScreenVideoRenderer};
#[cfg(not(target_arch = "wasm32"))]
use self::unsupported::UnavailableScreenVideoBackend as DefaultScreenVideoBackend;
#[cfg(target_arch = "wasm32")]
use self::web::WebScreenVideoBackend as DefaultScreenVideoBackend;
use super::realtime::InboundScreenFrame;

const SCREEN_VIDEO_RELEASE_TIMEOUT_MS: u32 = 1_500;

type ScreenVideoSubscribers =
    Rc<RefCell<HashMap<String, Vec<mpsc::UnboundedSender<InboundScreenFrame>>>>>;
type ScreenVideoGenerations = Rc<RefCell<HashMap<String, u64>>>;

/// Per-user screen sharing activity marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScreenVideoActivity {
    user_id: String,
}

/// Feature-local screen sharing video state.
#[derive(Clone)]
pub(crate) struct ScreenVideoHandle {
    live_users: Signal<Vec<ScreenVideoActivity>>,
    subscribers: ScreenVideoSubscribers,
    generations: ScreenVideoGenerations,
    backend: Rc<dyn ScreenVideoBackend>,
}

impl ScreenVideoHandle {
    /// Builds screen sharing video state.
    pub(crate) fn new(
        live_users: Signal<Vec<ScreenVideoActivity>>,
        subscribers: ScreenVideoSubscribers,
        generations: ScreenVideoGenerations,
    ) -> Self {
        Self {
            live_users,
            subscribers,
            generations,
            backend: Rc::new(DefaultScreenVideoBackend),
        }
    }

    /// Returns user identifiers currently publishing screen frames.
    pub(crate) fn live_user_ids(&self) -> Vec<String> {
        (self.live_users)()
            .into_iter()
            .map(|activity| activity.user_id)
            .collect()
    }

    /// Publishes an inbound screen frame to the matching participant tile.
    pub(crate) fn publish_frame(&self, frame: InboundScreenFrame) {
        self.mark_live(frame.sender_user_id.clone());
        let mut subscribers = self.subscribers.borrow_mut();
        let Some(user_subscribers) = subscribers.get_mut(&frame.sender_user_id) else {
            return;
        };
        user_subscribers.retain(|subscriber| subscriber.unbounded_send(frame.clone()).is_ok());
    }

    /// Subscribes a participant tile to one user's screen sharing frames.
    pub(crate) fn subscribe_user(
        &self,
        user_id: String,
    ) -> mpsc::UnboundedReceiver<InboundScreenFrame> {
        let (sender, receiver) = mpsc::unbounded();
        self.subscribers
            .borrow_mut()
            .entry(user_id)
            .or_default()
            .push(sender);

        receiver
    }

    /// Clears live screen sharing indicators.
    pub(crate) fn clear(&self) {
        self.generations.borrow_mut().clear();
        let mut live_users = self.live_users;
        live_users.set(Vec::new());
    }

    fn create_renderer(
        &self,
        target_id: String,
        user_id: String,
    ) -> Result<Rc<dyn ScreenVideoRenderer>, backend::ScreenVideoRenderError> {
        self.backend.create_renderer(target_id, user_id)
    }

    fn mark_live(&self, user_id: String) {
        let generation = {
            let mut generations = self.generations.borrow_mut();
            let generation = generations.entry(user_id.clone()).or_insert(0);
            *generation = generation.saturating_add(1);
            *generation
        };

        let mut next_users = (self.live_users)();
        if next_users
            .iter()
            .any(|activity| activity.user_id == user_id)
        {
            return;
        }
        next_users.push(ScreenVideoActivity {
            user_id: user_id.clone(),
        });
        let mut live_users = self.live_users;
        live_users.set(next_users);

        let generations = self.generations.clone();
        spawn(async move {
            let mut observed_generation = generation;
            loop {
                TimeoutFuture::new(SCREEN_VIDEO_RELEASE_TIMEOUT_MS).await;
                let latest_generation = generations.borrow().get(&user_id).copied();
                match latest_generation {
                    Some(latest_generation) if latest_generation != observed_generation => {
                        observed_generation = latest_generation;
                    }
                    Some(_) => {
                        generations.borrow_mut().remove(&user_id);
                        let mut next_users = live_users();
                        let previous_len = next_users.len();
                        next_users.retain(|activity| activity.user_id != user_id);
                        if next_users.len() != previous_len {
                            live_users.set(next_users);
                        }
                        break;
                    }
                    None => break,
                }
            }
        });
    }
}

/// Renders decoded screen sharing video for one participant.
#[component]
pub(crate) fn ScreenVideoCanvas(user_id: String) -> Element {
    let video = use_context::<ScreenVideoHandle>();
    let target_id = use_hook(|| format!("screen-video-{}", Uuid::new_v4().simple()));
    let mut waiting_for_key_frame = use_signal(|| true);

    use_hook({
        let target_id = target_id.clone();
        let user_id = user_id.clone();
        move || {
            spawn(async move {
                let mut frames = video.subscribe_user(user_id.clone());
                let renderer = match video.create_renderer(target_id.clone(), user_id.clone()) {
                    Ok(renderer) => renderer,
                    Err(error) => {
                        warn!(
                            %error,
                            %user_id,
                            "failed to create screen sharing video renderer"
                        );
                        return;
                    }
                };

                while let Some(frame) = frames.next().await {
                    if frame.key_frame {
                        waiting_for_key_frame.set(false);
                    }
                    if let Err(error) = renderer.decode(&frame) {
                        warn!(
                            %error,
                            sender_user_id = %frame.sender_user_id,
                            sequence = frame.sequence,
                            "failed to render screen sharing frame"
                        );
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
            "aria-hidden": "true",
            if waiting_for_key_frame() {
                div {
                    class: "pointer-events-none absolute inset-0 z-10 flex items-center justify-center bg-zinc-950/70",
                    div {
                        class: "h-8 w-8 animate-spin rounded-full border-2 border-zinc-700 border-t-sky-300",
                        "aria-hidden": "true",
                    }
                }
            }
        }
    }
}
