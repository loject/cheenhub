//! Social links component.

use dioxus::prelude::*;

#[component]
pub(crate) fn SocialLinks(class_name: &'static str, hover_class: &'static str) -> Element {
    rsx! {
        div { class: "{class_name}",
            a { href: "https://github.com/loject/cheenhub", target: "_blank", rel: "noopener", class: "{hover_class}", "GitHub" }
            a { href: "https://www.youtube.com/@cheengeez", target: "_blank", rel: "noopener", class: "{hover_class}", "YouTube" }
            a { href: "https://rutube.ru/channel/79753199", target: "_blank", rel: "noopener", class: "{hover_class}", "RuTube" }
            a { href: "https://t.me/cheenhub_official", target: "_blank", rel: "noopener", class: "{hover_class}", "Telegram" }
        }
    }
}
