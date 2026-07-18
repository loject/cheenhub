//! Страница оболочки аутентифицированного приложения.

use dioxus::prelude::*;

use crate::Route;
use crate::features::app::active_room::ActiveRoomContext;
use crate::features::app::components::app_shell::AppShell;
use crate::features::app::current_user::CurrentUserContext;
use crate::features::app::workspace_route::AppWorkspaceRoute;
use crate::features::app::workspace_route_storage;
use crate::features::audio_playback::AudioPlaybackProvider;
use crate::features::auth::{SessionEnd, TokenRefresher, api};
use crate::features::camera::CameraProvider;
use crate::features::microphone::MicrophoneProvider;
use crate::features::notifications::NotificationsProvider;
use crate::features::realtime::RealtimeProvider;
use crate::features::runtime::sleep_ms;
use crate::features::screen_share::ScreenShareProvider;
use crate::features::toast::ToastHandle;
use crate::features::voice_chat::VoiceConnectionProvider;

/// Рендерит домашнюю страницу вошедшего в систему приложения.
#[component]
pub(crate) fn AppPage() -> Element {
    let navigator = use_navigator();
    let toast = use_context::<ToastHandle>();
    let route = use_route::<Route>();
    let user = use_signal(|| None);
    let mut loading_profile = use_signal(|| false);
    let mut profile_error = use_signal(|| None::<String>);
    let mut load_attempt = use_signal(|| 0_u32);
    let mut started_attempt = use_signal(|| None::<u32>);
    let mut saved_workspace_route = use_signal(|| None::<(String, String)>);
    let current_user_context = CurrentUserContext::new(user);
    use_context_provider(move || current_user_context);
    // Контекст активной комнаты для фильтрации уведомлений.
    let active_room_id = use_signal(|| None::<String>);
    let active_conversation_id = use_signal(|| None::<String>);
    let active_room_context = ActiveRoomContext::new(active_room_id, active_conversation_id);
    use_context_provider(move || active_room_context);

    use_effect(move || {
        if !api::has_tokens() {
            info!("redirecting unauthenticated app route to login");
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
                Err(error) if api::has_tokens() => {
                    let mut profile_error = profile_error;
                    let mut loading_profile = loading_profile;
                    let mut load_attempt = load_attempt;
                    warn!(%error, "current user load deferred while saved session remains available");
                    profile_error.set(Some(error));
                    loading_profile.set(false);
                    sleep_ms(5_000).await;
                    load_attempt.with_mut(|attempt| *attempt = attempt.saturating_add(1));
                }
                Err(error) => {
                    warn!("current user load failed; redirecting to login: {error}");
                    toast.session_error(error);
                    let _ = navigator.replace(Route::Login {});
                }
            }
        });
    });

    use_effect(move || {
        let Some(current_user) = user() else {
            return;
        };

        match route.clone() {
            Route::AppHome {} => {
                let saved_route = workspace_route_storage::load(&current_user.id);
                let next_route = saved_route.unwrap_or(Route::AppFriends {});
                info!(
                    route = %next_route,
                    "restoring authenticated app workspace route"
                );
                navigator.replace(next_route);
            }
            _ if AppWorkspaceRoute::from_route(&route).is_some() => {
                let route_text = route.to_string();
                let saved_route_key = (current_user.id.clone(), route_text.clone());
                if saved_workspace_route() == Some(saved_route_key.clone()) {
                    return;
                }

                debug!(
                    user_id = %current_user.id,
                    route = %route_text,
                    "saving authenticated app workspace route"
                );
                workspace_route_storage::save(&current_user.id, &route);
                saved_workspace_route.set(Some(saved_route_key));
            }
            _ => {}
        }
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
            on_session_expired: move |session_end: SessionEnd| {
                warn!(reason = ?session_end.reason, "showing auth session end reason");
                toast.session_error(session_end.message);
                let _ = navigator.replace(Route::Login {});
            },
        }
        RealtimeProvider {
            AudioPlaybackProvider {
                MicrophoneProvider {
                    CameraProvider {
                        ScreenShareProvider {
                            VoiceConnectionProvider {
                                NotificationsProvider {
                                    AppShell {}
                                    Outlet::<Route> {}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
