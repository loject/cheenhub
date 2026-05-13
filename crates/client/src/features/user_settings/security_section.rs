//! User security settings section.

use dioxus::prelude::*;

use super::password_section::PasswordSettingsSection;

#[derive(Clone, PartialEq)]
struct AuthorizedDevice {
    id: &'static str,
    name: &'static str,
    location: &'static str,
    last_active: &'static str,
    current: bool,
}

/// Renders mock authorized device security controls.
#[component]
pub(crate) fn SecuritySettingsSection() -> Element {
    let mut devices = use_signal(mock_devices);
    let active_devices = devices();
    let other_device_count = active_devices
        .iter()
        .filter(|device| !device.current)
        .count();

    rsx! {
        div { class: "space-y-4",
            PasswordSettingsSection {}
            div { class: "rounded-2xl border border-zinc-800 bg-zinc-950/70 p-4",
                div { class: "flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between",
                    div {
                        h3 { class: "text-[16px] font-semibold tracking-[-0.03em] text-zinc-50", "Авторизованные устройства" }
                        p { class: "mt-1 text-[12px] leading-5 text-zinc-500", "Устройства, где сейчас открыт аккаунт." }
                    }
                    button {
                        r#type: "button",
                        disabled: other_device_count == 0,
                        class: "flex h-9 shrink-0 items-center justify-center rounded-xl border border-red-500/25 bg-red-500/10 px-3 text-[12px] font-medium text-red-200 transition hover:border-red-500/35 hover:bg-red-500/15 disabled:cursor-not-allowed disabled:opacity-50",
                        onclick: move |_| {
                            let current_only = devices()
                                .into_iter()
                                .filter(|device| device.current)
                                .collect::<Vec<_>>();
                            devices.set(current_only);
                        },
                        "Выйти со всех устройств"
                    }
                }

                if active_devices.is_empty() {
                    div { class: "mt-4 rounded-2xl border border-zinc-800 bg-zinc-900/45 p-4",
                        p { class: "text-[13px] font-medium text-zinc-100", "Устройств нет" }
                        p { class: "mt-1 text-[12px] leading-5 text-zinc-500", "Активные сеансы появятся здесь после входа." }
                    }
                } else {
                    div { class: "mt-4 space-y-2",
                        for device in active_devices {
                            div {
                                key: "{device.id}",
                                class: "flex flex-col gap-3 rounded-2xl border border-zinc-800 bg-zinc-900/45 p-3 sm:flex-row sm:items-center sm:justify-between",
                                div { class: "min-w-0 flex items-center gap-3",
                                    div { class: device_icon_class(device.current),
                                        svg { class: "h-4 w-4", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9 17.25v1.007a3 3 0 0 1-.879 2.122L7.5 21h9l-.621-.621A3 3 0 0 1 15 18.257V17.25m-9.75 0h13.5A2.25 2.25 0 0 0 21 15V5.25A2.25 2.25 0 0 0 18.75 3H5.25A2.25 2.25 0 0 0 3 5.25V15a2.25 2.25 0 0 0 2.25 2.25Z" }
                                        }
                                    }
                                    div { class: "min-w-0",
                                        div { class: "flex min-w-0 items-center gap-2",
                                            p { class: "truncate text-[13px] font-medium text-zinc-100", "{device.name}" }
                                            if device.current {
                                                span { class: "shrink-0 rounded-full border border-accent/25 bg-accent/10 px-2 py-0.5 text-[10px] font-medium text-blue-200", "текущее" }
                                            }
                                        }
                                        p { class: "mt-0.5 truncate text-[11px] text-zinc-500", "{device.location} · {device.last_active}" }
                                    }
                                }
                                if device.current {
                                    span { class: "self-start rounded-xl border border-zinc-800 px-3 py-2 text-[12px] text-zinc-500 sm:self-auto", "Активно" }
                                } else {
                                    button {
                                        r#type: "button",
                                        class: "flex h-9 shrink-0 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[12px] font-medium text-zinc-300 transition hover:border-red-500/35 hover:bg-red-500/10 hover:text-red-200",
                                        onclick: move |_| {
                                            devices.set(
                                                devices()
                                                    .into_iter()
                                                    .filter(|active_device| active_device.id != device.id)
                                                    .collect::<Vec<_>>(),
                                            );
                                        },
                                        "Выйти"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn mock_devices() -> Vec<AuthorizedDevice> {
    vec![
        AuthorizedDevice {
            id: "current-linux",
            name: "Linux Desktop · Firefox",
            location: "Новосибирск, RU",
            last_active: "сейчас",
            current: true,
        },
        AuthorizedDevice {
            id: "macbook-safari",
            name: "MacBook Pro · Safari",
            location: "Москва, RU",
            last_active: "2 часа назад",
            current: false,
        },
        AuthorizedDevice {
            id: "iphone-app",
            name: "iPhone 15 · CheenHub",
            location: "Санкт-Петербург, RU",
            last_active: "вчера",
            current: false,
        },
        AuthorizedDevice {
            id: "windows-edge",
            name: "Windows PC · Edge",
            location: "Екатеринбург, RU",
            last_active: "4 дня назад",
            current: false,
        },
    ]
}

fn device_icon_class(current: bool) -> &'static str {
    if current {
        "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-accent/25 bg-accent/10 text-blue-200"
    } else {
        "flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 text-zinc-500"
    }
}
