//! Native client download link component.

use dioxus::prelude::*;

#[component]
pub(crate) fn DownloadLink(
    href: Option<&'static str>,
    label: &'static str,
    format: &'static str,
    disabled: bool,
    status: Option<&'static str>,
) -> Element {
    let status_text = status.unwrap_or(format);

    if disabled {
        return rsx! {
            button {
                r#type: "button",
                disabled: true,
                title: "{label}: клиент в разработке",
                class: "flex w-full cursor-not-allowed items-center justify-between rounded-xl px-3 py-2 text-left text-[13px] text-zinc-500 opacity-70",
                span { "{label}" }
                span { class: "ml-4 shrink-0 text-[11px] text-amber-300/80", "{status_text}" }
            }
        };
    }

    rsx! {
        a {
            href: href.unwrap_or("#"),
            class: "flex items-center justify-between rounded-xl px-3 py-2 text-[13px] text-zinc-300 transition hover:bg-zinc-900 hover:text-zinc-100",
            span { "{label}" }
            span { class: "ml-4 shrink-0 text-[11px] text-zinc-600", "{status_text}" }
        }
    }
}
