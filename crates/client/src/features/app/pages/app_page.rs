//! Authenticated application shell page.

use dioxus::prelude::*;

use crate::Route;
use crate::features::app::components::app_shell::AppShell;
use crate::features::audio_playback::AudioPlaybackProvider;
use crate::features::auth::{TokenRefresher, api};
use crate::features::microphone::MicrophoneProvider;
use crate::features::realtime::RealtimeProvider;
use crate::features::voice_chat::VoiceConnectionProvider;

/// Renders the signed-in application home.
#[component]
pub(crate) fn AppPage() -> Element {
    let navigator = use_navigator();
    let user = use_signal(|| None);
    let mut loaded_profile = use_signal(|| false);

    use_effect(move || {
        if !api::has_tokens() {
            navigator.replace(Route::Login {});
            return;
        }
        if loaded_profile() {
            return;
        }
        loaded_profile.set(true);

        spawn(async move {
            match api::current_user().await {
                Ok(current_user) => {
                    let mut user = user;
                    user.set(Some(current_user));
                }
                Err(_) => {
                    let _ = navigator.replace(Route::Login {});
                }
            }
        });
    });

    let Some(current_user) = user() else {
        return rsx! {
            div { class: "grid min-h-screen place-items-center bg-zinc-950 px-5 text-zinc-300",
                "Открываем CheenHub..."
            }
        };
    };

    rsx! {
        TokenRefresher {
            on_session_expired: move |_| {
                let _ = navigator.replace(Route::Login {});
            },
        }
        RealtimeProvider {
            AudioPlaybackProvider {
                MicrophoneProvider {
                    VoiceConnectionProvider { current_user,
                        AppShell {}
                    }
                }
            }
        }
    }
}
