//! Social links component.

use dioxus::prelude::*;

use crate::features::landing::components::todo_button::TodoButton;

#[component]
pub(crate) fn SocialLinks(class_name: &'static str, hover_class: &'static str) -> Element {
    rsx! {
        div { class: "{class_name}",
            a { href: "https://github.com/loject/cheenhub", target: "_blank", rel: "noopener", class: "{hover_class}", "GitHub" }
            a { href: "https://youtube.com/@cheenhub", target: "_blank", rel: "noopener", class: "{hover_class}", "YouTube" }
            TodoButton { class_name: hover_class, label: "RuTube" }
            TodoButton { class_name: hover_class, label: "Telegram" }
        }
    }
}
