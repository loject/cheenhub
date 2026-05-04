//! Feature icon renderer component.

use dioxus::prelude::*;

use crate::features::landing::data::FeatureIcon;

#[component]
pub(crate) fn FeatureSvg(icon: FeatureIcon) -> Element {
    match icon {
        FeatureIcon::Phone => rsx! {
            svg { class: "h-[18px] w-[18px]", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M3 5a2 2 0 0 1 2-2h3.28a1 1 0 0 1 .948.684l1.498 4.493a1 1 0 0 1-.502 1.21l-2.257 1.13a11.042 11.042 0 0 0 5.516 5.516l1.13-2.257a1 1 0 0 1 1.21-.502l4.493 1.498a1 1 0 0 1 .684.949V19a2 2 0 0 1-2 2h-1C9.716 21 3 14.284 3 6V5Z" }
            }
        },
        FeatureIcon::Users => rsx! {
            svg { class: "h-[18px] w-[18px]", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M17 20h5v-2a3 3 0 0 0-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 0 1 5.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 0 1 9.288 0M15 7a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z" }
            }
        },
        FeatureIcon::Screen => rsx! {
            svg { class: "h-[18px] w-[18px]", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24",
                rect { x: "2", y: "3", width: "20", height: "14", rx: "2" }
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M8 21h8m-4-4v4" }
            }
        },
        FeatureIcon::Code => rsx! {
            svg { class: "h-[18px] w-[18px]", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M10 20l4-16m4 4 4 4-4 4M6 16l-4-4 4-4" }
            }
        },
        FeatureIcon::Shield => rsx! {
            svg { class: "h-[18px] w-[18px]", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10Z" }
            }
        },
        FeatureIcon::CheckShield => rsx! {
            svg { class: "h-[18px] w-[18px]", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "m9 12 2 2 4-4m5.618-4.016A11.955 11.955 0 0 1 12 2.944a11.955 11.955 0 0 1-8.618 3.04A12.02 12.02 0 0 0 3 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016Z" }
            }
        },
    }
}
