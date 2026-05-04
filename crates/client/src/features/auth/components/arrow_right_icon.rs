//! Right arrow icon component.

use dioxus::prelude::*;

#[component]
pub(crate) fn ArrowRightIcon(class_name: &'static str) -> Element {
    rsx! {
        svg { class: "{class_name}", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M5 12h14m0 0-6-6m6 6-6 6" }
        }
    }
}
