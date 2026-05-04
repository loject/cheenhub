//! Native client download dropdown component.

use dioxus::prelude::*;

use crate::features::landing::components::chevron_down_icon::ChevronDownIcon;
use crate::features::landing::components::download_icon::DownloadIcon;
use crate::features::landing::components::download_link::DownloadLink;

#[component]
pub(crate) fn DownloadDropdown(opens_up: bool, large: bool) -> Element {
    let mut is_open = use_signal(|| false);
    let button_class = if large {
        "btn-g flex items-center gap-2 rounded-xl border border-zinc-700 bg-zinc-900 px-6 py-3 text-[13px] font-semibold text-zinc-200"
    } else {
        "btn-g flex items-center gap-2 rounded-xl border border-zinc-700 bg-zinc-900 px-5 py-2.5 text-[13px] font-medium text-zinc-200"
    };
    let menu_class = match (opens_up, is_open()) {
        (true, true) => {
            "absolute left-0 bottom-full z-[80] mb-2 min-w-[220px] overflow-hidden rounded-2xl border border-zinc-800 bg-zinc-950/95 p-1.5 text-left shadow-[0_18px_50px_rgba(0,0,0,0.45)] backdrop-blur-xl"
        }
        (true, false) => {
            "absolute left-0 bottom-full z-[80] mb-2 hidden min-w-[220px] overflow-hidden rounded-2xl border border-zinc-800 bg-zinc-950/95 p-1.5 text-left shadow-[0_18px_50px_rgba(0,0,0,0.45)] backdrop-blur-xl"
        }
        (false, true) => {
            "absolute left-0 top-full z-[80] mt-2 min-w-[220px] overflow-hidden rounded-2xl border border-zinc-800 bg-zinc-950/95 p-1.5 shadow-[0_18px_50px_rgba(0,0,0,0.45)] backdrop-blur-xl"
        }
        (false, false) => {
            "absolute left-0 top-full z-[80] mt-2 hidden min-w-[220px] overflow-hidden rounded-2xl border border-zinc-800 bg-zinc-950/95 p-1.5 shadow-[0_18px_50px_rgba(0,0,0,0.45)] backdrop-blur-xl"
        }
    };
    let expanded = if is_open() { "true" } else { "false" };

    rsx! {
        div { class: "relative z-[70]",
            button {
                r#type: "button",
                aria_expanded: "{expanded}",
                class: "{button_class}",
                onclick: move |_| is_open.set(!is_open()),
                DownloadIcon { class_name: "h-4 w-4" }
                "Скачать нативный клиент"
                ChevronDownIcon { class_name: "h-3.5 w-3.5 text-zinc-500" }
            }
            div { class: "{menu_class}",
                DownloadLink { href: "https://cheenhub.ru/download/windows", label: "Windows", format: ".msi" }
                DownloadLink { href: "https://cheenhub.ru/download/linux", label: "Ubuntu/Linux", format: ".deb" }
                DownloadLink { href: "https://cheenhub.ru/download/android", label: "Android", format: ".apk" }
            }
        }
    }
}
