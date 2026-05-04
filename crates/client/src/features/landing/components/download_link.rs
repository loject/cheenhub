//! Native client download link component.

use dioxus::prelude::*;

#[component]
pub(crate) fn DownloadLink(
    href: &'static str,
    label: &'static str,
    format: &'static str,
) -> Element {
    rsx! {
        a {
            href,
            class: "flex items-center justify-between rounded-xl px-3 py-2 text-[13px] text-zinc-300 transition hover:bg-zinc-900 hover:text-zinc-100",
            span { "{label}" }
            span { class: "text-[11px] text-zinc-600", "{format}" }
        }
    }
}
