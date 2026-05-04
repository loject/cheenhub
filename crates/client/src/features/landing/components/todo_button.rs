//! Placeholder action button component.

use dioxus::prelude::*;

use crate::features::landing::behavior::show_todo_alert;

#[component]
pub(crate) fn TodoButton(class_name: &'static str, label: &'static str) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: "{class_name}",
            onclick: move |_| show_todo_alert(),
            "{label}"
        }
    }
}
