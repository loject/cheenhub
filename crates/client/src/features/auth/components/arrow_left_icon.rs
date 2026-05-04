//! Left arrow icon component.

use dioxus::prelude::*;

#[component]
pub(crate) fn ArrowLeftIcon(class_name: &'static str) -> Element {
    rsx! {
        svg { class: "{class_name}", fill: "none", stroke: "currentColor", stroke_width: "2", view_box: "0 0 24 24",
            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19 12H5m0 0 6 6m-6-6 6-6" }
        }
    }
}
