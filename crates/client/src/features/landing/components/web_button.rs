//! Web client link button component.

use dioxus::prelude::*;

use crate::features::landing::components::globe_icon::GlobeIcon;

#[component]
pub(crate) fn WebButton(large: bool) -> Element {
    let class_name = if large {
        "btn-p flex items-center gap-2 rounded-xl bg-accent px-6 py-3 text-[13px] font-semibold text-white shadow-[0_0_0_1px_rgba(59,130,246,0.3),0_8px_28px_rgba(59,130,246,0.18)]"
    } else {
        "btn-p flex items-center gap-2 rounded-xl bg-accent px-5 py-2.5 text-[13px] font-medium text-white shadow-[0_0_0_1px_rgba(59,130,246,0.3),0_4px_18px_rgba(59,130,246,0.16)]"
    };

    rsx! {
        a {
            href: "https://cheenhub.ru/web",
            target: "_blank",
            rel: "noopener",
            class: "{class_name}",
            GlobeIcon { class_name: "h-4 w-4" }
            "Открыть веб-версию"
        }
    }
}
