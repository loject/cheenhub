//! Web-реализация блока скачивания native-клиента в серверной панели.

use dioxus::prelude::*;

use crate::features::landing::components::download_icon::DownloadIcon;
use crate::features::landing::components::download_link::DownloadLink;
use crate::features::landing::data::{app_version, windows_installer_url};

/// Рендерит компактный выбор OS для скачивания native-клиента.
#[component]
pub(crate) fn NativeClientDownload() -> Element {
    let mut is_open = use_signal(|| false);
    let app_version = app_version();
    let windows_installer_url = windows_installer_url();
    let menu_class = if is_open() {
        "absolute bottom-0 left-[calc(100%+12px)] z-[95] min-w-[220px] overflow-hidden rounded-2xl border border-zinc-800 bg-zinc-950/95 p-1.5 text-left shadow-[0_18px_50px_rgba(0,0,0,0.45)] backdrop-blur-xl"
    } else {
        "absolute bottom-0 left-[calc(100%+12px)] z-[95] hidden min-w-[220px] overflow-hidden rounded-2xl border border-zinc-800 bg-zinc-950/95 p-1.5 text-left shadow-[0_18px_50px_rgba(0,0,0,0.45)] backdrop-blur-xl"
    };
    let expanded = if is_open() { "true" } else { "false" };

    let install_pwa = move |_| {
        info!("requesting PWA install from authenticated app server rail");
        document::eval(
            r#"
            window.dispatchEvent(new CustomEvent("cheenhub:pwa-install"));
            "#,
        );
    };

    rsx! {
        div { class: "relative mt-2",
            button {
                r#type: "button",
                aria_expanded: "{expanded}",
                "aria-label": "Скачать native-клиент",
                title: "Скачать native-клиент",
                class: "flex h-12 w-12 items-center justify-center rounded-2xl border border-zinc-800 bg-zinc-900/80 text-zinc-400 transition-[background,border-color,color,transform,opacity] duration-150 hover:-translate-y-px hover:border-blue-400/30 hover:bg-blue-500/10 hover:text-zinc-100",
                onclick: move |_| {
                    let next_open = !is_open();
                    info!(open = next_open, "toggling authenticated app native client download menu");
                    is_open.set(next_open);
                },
                DownloadIcon { class_name: "h-5 w-5" }
            }

            div { class: "{menu_class}",
                p { class: "px-3 pb-1.5 pt-1 text-[11px] font-semibold uppercase tracking-[0.16em] text-zinc-500",
                    "Native-клиент"
                }

                button {
                    r#type: "button",
                    class: "flex w-full items-center justify-between rounded-xl px-3 py-2 text-left text-[13px] text-zinc-300 transition hover:bg-zinc-900 hover:text-zinc-100",
                    onclick: install_pwa,
                    span { "Установить PWA" }
                    span { class: "ml-4 shrink-0 text-[11px] text-blue-300", "web" }
                }

                div { class: "my-1 h-px bg-zinc-800/80" }

                a {
                    href: "{windows_installer_url}",
                    class: "flex items-center justify-between rounded-xl px-3 py-2 text-[13px] text-zinc-300 transition hover:bg-zinc-900 hover:text-zinc-100",
                    onclick: move |_| {
                        info!(
                            os = "windows",
                            app_version,
                            "starting native client download from authenticated app server rail"
                        );
                    },
                    span { "Windows" }
                    span { class: "ml-4 shrink-0 text-[11px] text-zinc-600", "{app_version}" }
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
