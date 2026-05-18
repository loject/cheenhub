//! Server avatar renderer.

use dioxus::prelude::*;

/// Renders a server avatar image with name initials fallback.
#[component]
pub(crate) fn ServerAvatar(name: String, avatar_url: Option<String>, class: String) -> Element {
    let mut image_failed = use_signal(|| false);
    let show_image = avatar_url.is_some() && !image_failed();
    let label = initials(&name);

    rsx! {
        div { class,
            if show_image {
                img {
                    class: "h-full w-full object-cover",
                    src: avatar_url.unwrap_or_default(),
                    alt: "",
                    onerror: move |_| image_failed.set(true),
                }
            } else {
                span { "{label}" }
            }
        }
    }
}

fn initials(name: &str) -> String {
    let mut initials = name
        .split_whitespace()
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase();

    if initials.is_empty() {
        initials = name.chars().take(2).collect::<String>().to_uppercase();
    }

    initials
}
