//! Authenticated application shell page.

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::Route;
use crate::features::app::components::app_shell::AppShell;
use crate::features::app::current_user::CurrentUserContext;
use crate::features::audio_playback::AudioPlaybackProvider;
use crate::features::auth::{TokenRefresher, api};
use crate::features::microphone::MicrophoneProvider;
use crate::features::realtime::RealtimeProvider;
use crate::features::screen_share::ScreenShareProvider;
use crate::features::voice_chat::VoiceConnectionProvider;

/// Renders the signed-in application home.
#[component]
pub(crate) fn AppPage() -> Element {
    let navigator = use_navigator();
    let user = use_signal(|| None);
    let mut loading_profile = use_signal(|| false);
    let mut profile_error = use_signal(|| None::<String>);
    let mut load_attempt = use_signal(|| 0_u32);
    let mut started_attempt = use_signal(|| None::<u32>);
    let current_user_context = CurrentUserContext::new(user);
    use_context_provider(move || current_user_context);

    use_effect(move || {
        if !api::has_tokens() {
            navigator.replace(Route::Login {});
            return;
        }
        if user().is_some() || loading_profile() {
            return;
        }

        let attempt = load_attempt();
        if started_attempt() == Some(attempt) {
            return;
        }

        started_attempt.set(Some(attempt));
        loading_profile.set(true);

        spawn(async move {
            match api::current_user().await {
                Ok(current_user) => {
                    let mut user = user;
                    let mut profile_error = profile_error;
                    let mut loading_profile = loading_profile;
                    info!("current user loaded for authenticated app");
                    user.set(Some(current_user));
                    profile_error.set(None);
                    loading_profile.set(false);
                }
                Err(error) if api::is_network_error(&error) && api::has_tokens() => {
                    let mut profile_error = profile_error;
                    let mut loading_profile = loading_profile;
                    let mut load_attempt = load_attempt;
                    warn!("current user load deferred while network is unavailable");
                    profile_error.set(Some(error));
                    loading_profile.set(false);
                    TimeoutFuture::new(5_000).await;
                    load_attempt.with_mut(|attempt| *attempt = attempt.saturating_add(1));
                }
                Err(error) => {
                    warn!("current user load failed; redirecting to login: {error}");
                    let _ = navigator.replace(Route::Login {});
                }
            }
        });
    });

    if user().is_none() {
        if profile_error().is_some() {
            let retrying = loading_profile();
            return rsx! {
                div { class: "grid min-h-screen place-items-center bg-zinc-950 px-5 text-zinc-100",
                    section { class: "w-full max-w-md rounded-lg border border-blue-400/20 bg-zinc-900/90 p-7 shadow-2xl shadow-black/30",
                        p { class: "text-sm font-semibold uppercase text-blue-300", "Нет соединения" }
                        h1 { class: "mt-2 text-2xl font-bold text-white", "CheenHub ждёт интернет" }
                        p { class: "mt-3 text-sm leading-6 text-zinc-300",
                            "Сессия сохранена на этом устройстве. Мы не выходим из аккаунта и продолжим загрузку профиля, когда сервер станет доступен."
                        }
                        button {
                            class: "mt-6 min-h-11 rounded-lg bg-blue-400 px-4 font-bold text-zinc-950 transition hover:bg-blue-300 disabled:cursor-wait disabled:opacity-70",
                            disabled: retrying,
                            onclick: move |_| {
                                profile_error.set(None);
                                load_attempt.with_mut(|attempt| *attempt = attempt.saturating_add(1));
                            },
                            if retrying { "Проверяем..." } else { "Проверить соединение" }
                        }
                    }
                }
            };
        }

        return rsx! {
            div { class: "grid min-h-screen place-items-center bg-zinc-950 px-5 text-zinc-300",
                "Открываем CheenHub..."
            }
        };
    }

    rsx! {
        TokenRefresher {
            on_session_expired: move |_| {
                let _ = navigator.replace(Route::Login {});
            },
        }
        RealtimeProvider {
            AudioPlaybackProvider {
                MicrophoneProvider {
                    ScreenShareProvider {
                        VoiceConnectionProvider {
                            AppShell {}
                        }
                    }
                }
            }
        }
    }
}
