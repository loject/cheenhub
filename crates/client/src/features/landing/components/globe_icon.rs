//! Globe icon component.

use dioxus::prelude::*;

#[component]
pub(crate) fn GlobeIcon(class_name: &'static str) -> Element {
    rsx! {
        svg { class: "{class_name}", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24",
            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M3 12h18M12 3c2.5 2.5 4 5.75 4 9s-1.5 6.5-4 9m0-18c-2.5 2.5-4 5.75-4 9s1.5 6.5 4 9" }
        }
    }
}
