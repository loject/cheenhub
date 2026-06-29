//! Native client download dropdown component.

use dioxus::prelude::*;

use crate::features::landing::components::chevron_down_icon::ChevronDownIcon;
use crate::features::landing::components::download_icon::DownloadIcon;
use crate::features::landing::components::download_link::DownloadLink;

#[component]
pub(crate) fn DownloadDropdown(opens_up: bool, large: bool) -> Element {
    let app_version = env!("CHEENHUB_APP_VERSION");

    let release_version = if app_version.starts_with('v') {
        app_version.to_string()
    } else {
        format!("v{app_version}")
    };

    let windows_installer_url = format!(
        "https://github.com/loject/cheenhub/releases/latest/download/cheenhub-{release_version}-windows-x64-setup.exe"
    );

    let mut is_open = use_signal(|| false);

    let install_pwa = move |_| {
        document::eval(
            r#"
            window.dispatchEvent(new CustomEvent("cheenhub:pwa-install"));
            "#,
        );
    };

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
                    href: None,
                    label: "Ubuntu/Linux",
                    format: ".deb",
                    disabled: true,
                    status: Some("в разработке"),
                }

                DownloadLink {
                    href: None,
                    label: "Android",
                    format: ".apk",
                    disabled: true,
                    status: Some("в разработке"),
                }
            }
        }
    }
}
