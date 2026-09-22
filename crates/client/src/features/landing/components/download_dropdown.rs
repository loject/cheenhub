//! Native client download dropdown component.

use dioxus::prelude::*;

use crate::features::landing::components::chevron_down_icon::ChevronDownIcon;
use crate::features::landing::components::download_icon::DownloadIcon;
use crate::features::landing::components::download_link::DownloadLink;
use crate::features::landing::data::{
    android_apk_url, app_version, linux_deb_url, windows_installer_url,
};

#[component]
pub(crate) fn DownloadDropdown(
    opens_up: bool,
    large: bool,
    compact: bool,
    opens_right: bool,
    #[props(default)] external_open: Option<Signal<bool>>,
) -> Element {
    let app_version = app_version();
    let android_apk_url = android_apk_url();
    let linux_deb_url = linux_deb_url();
    let windows_installer_url = windows_installer_url();
    let local_open = use_signal(|| false);
    let mut is_open = external_open.unwrap_or(local_open);

    let install_pwa = move |_| {
        document::eval(
            r#"
            window.dispatchEvent(new CustomEvent("cheenhub:pwa-install"));
            "#,
        );
    };

    let button_class = if compact {
        "flex h-12 w-12 items-center justify-center rounded-2xl border border-zinc-800 bg-zinc-900/80 text-zinc-400 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-blue-400/30 hover:bg-blue-500/10 hover:text-zinc-100"
    } else if large {
        "btn-g flex items-center gap-2 rounded-xl border border-zinc-700 bg-zinc-900 px-6 py-3 text-[13px] font-semibold text-zinc-200"
    } else {
        "btn-g flex items-center gap-2 rounded-xl border border-zinc-700 bg-zinc-900 px-5 py-2.5 text-[13px] font-medium text-zinc-200"
    };

    let menu_class = if opens_right {
        if is_open() {
            "absolute bottom-0 left-[calc(100%+12px)] z-[100] min-w-[220px] overflow-hidden rounded-2xl border border-zinc-800 bg-zinc-950/95 p-1.5 text-left shadow-[0_18px_50px_rgba(0,0,0,0.45)] backdrop-blur-xl"
        } else {
            "absolute bottom-0 left-[calc(100%+12px)] z-[100] hidden min-w-[220px] overflow-hidden rounded-2xl border border-zinc-800 bg-zinc-950/95 p-1.5 text-left shadow-[0_18px_50px_rgba(0,0,0,0.45)] backdrop-blur-xl"
        }
    } else {
        match (opens_up, is_open()) {
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
        }
    };

    let expanded = if is_open() { "true" } else { "false" };
    let wrapper_class = if compact && is_open() {
        "relative mt-2 z-[95]"
    } else if compact {
        "relative mt-2 z-[70]"
    } else if is_open() {
        "relative z-[95]"
    } else {
        "relative z-[70]"
    };
    let icon_class = if compact { "h-5 w-5" } else { "h-4 w-4" };

    rsx! {
        div { class: "{wrapper_class}",
            if is_open() && external_open.is_none() {
                div {
                    class: "fixed inset-0 z-[99] cursor-default",
                    "aria-label": "Закрыть меню загрузки",
                    onclick: move |_| is_open.set(false),
                }
            }
            button {
                r#type: "button",
                aria_expanded: "{expanded}",
                aria_label: "Скачать нативный клиент",
                class: "{button_class}",
                onclick: move |event| {
                    event.stop_propagation();
                    is_open.set(!is_open());
                },
                DownloadIcon { class_name: icon_class }
                if !compact {
                    "Скачать нативный клиент"
                    ChevronDownIcon { class_name: "h-3.5 w-3.5 text-zinc-500" }
                }
            }

            div { class: "{menu_class}",
                button {
                    r#type: "button",
                    class: "flex w-full items-center justify-between rounded-xl px-3 py-2 text-left text-[13px] text-zinc-300 transition hover:bg-zinc-900 hover:text-zinc-100",
                    onclick: install_pwa,
                    span { "Установить PWA" }
                    span { class: "ml-4 shrink-0 text-[11px] text-blue-300", "web" }
                }

                div { class: "my-1 h-px bg-zinc-800/80" }

                DownloadLink {
                    href: Some(windows_installer_url),
                    label: "Windows",
                    format: ".exe",
                    disabled: false,
                    status: Some(app_version),
                }

                DownloadLink {
                    href: Some(linux_deb_url),
                    label: "Ubuntu",
                    format: ".deb",
                    disabled: false,
                    status: None,
                }

                DownloadLink {
                    href: None,
                    label: "Linux AppImage",
                    format: ".AppImage",
                    disabled: true,
                    status: Some("скоро"),
                }

                DownloadLink {
                    href: Some(android_apk_url),
                    label: "Android",
                    format: ".apk",
                    disabled: false,
                    status: None,
                }

                DownloadLink {
                    href: Some("https://www.rustore.ru/catalog/app/ru.cheenhub".to_owned()),
                    label: "Android · RuStore",
                    format: "RuStore",
                    disabled: false,
                    status: None,
                }

                DownloadLink {
                    href: None,
                    label: "Google Play",
                    format: "Google Play",
                    disabled: true,
                    status: Some("скоро"),
                }

                div { class: "my-1 h-px bg-zinc-800/80" }

                DownloadLink {
                    href: Some("https://github.com/loject/cheenhub".to_owned()),
                    label: "Собрать из исходников",
                    format: "GitHub",
                    disabled: false,
                    status: None,
                }
            }
        }
    }
}
