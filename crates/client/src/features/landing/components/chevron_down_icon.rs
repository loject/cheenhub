//! Chevron down icon component.

use dioxus::prelude::*;

#[component]
pub(crate) fn ChevronDownIcon(class_name: &'static str) -> Element {
    rsx! {
        svg { class: "{class_name}", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
            path { stroke_linecap: "round", stroke_linejoin: "round", d: "m6 9 6 6 6-6" }
        }
    }
}
