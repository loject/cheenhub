//! Placeholder action button component.

use dioxus::prelude::*;

#[component]
pub(crate) fn TodoButton(class_name: &'static str, label: &'static str) -> Element {
    let mut message_visible = use_signal(|| false);

    rsx! {
        span { class: "inline-flex flex-col items-start gap-2",
            button {
                r#type: "button",
                class: "{class_name}",
                onclick: move |_| message_visible.set(true),
                "{label}"
            }
            if message_visible() {
                span { class: "text-xs font-medium text-zinc-500", "Ссылка появится позже." }
            }
        }
    }
}
