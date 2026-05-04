//! Download icon component.

use dioxus::prelude::*;

#[component]
pub(crate) fn DownloadIcon(class_name: &'static str) -> Element {
    rsx! {
        svg { class: "{class_name}", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24",
            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 16V4m0 12 4-4m-4 4-4-4M5 20h14" }
        }
    }
}
