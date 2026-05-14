//! User avatar renderer.

use dioxus::prelude::*;

/// Renders a user avatar image with nickname initial fallback.
#[component]
pub(crate) fn UserAvatar(nickname: String, avatar_url: Option<String>, class: String) -> Element {
    let mut image_failed = use_signal(|| false);
    let initial = nickname_initial(&nickname);
    let show_image = avatar_url.is_some() && !image_failed();

    rsx! {
        div { class: "{class} relative overflow-hidden",
            if show_image {
                img {
                    class: "absolute inset-0 h-full w-full object-cover",
                    src: avatar_url.unwrap_or_default(),
                    alt: "{nickname}",
                    onerror: move |_| image_failed.set(true),
                }
            } else {
                div { class: "absolute inset-0 flex items-center justify-center",
                    "{initial}"
                }
            }
        }
    }
}

fn nickname_initial(nickname: &str) -> String {
    nickname
        .chars()
        .next()
        .map(|letter| letter.to_uppercase().collect())
        .unwrap_or_else(|| "?".to_owned())
}
