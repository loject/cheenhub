//! Компонент маршрута "не найдено".

use dioxus::prelude::*;

use crate::Route;
use crate::features::auth::api;
use crate::features::landing;
use crate::features::landing::components::logo_icon::LogoIcon;

/// Рендерит запасную страницу для неизвестных клиентских маршрутов.
#[component]
pub(crate) fn NotFound(route: Vec<String>) -> Element {
    let requested_path = format!("/{}", route.join("/"));
    let has_session = api::has_tokens();
    let home_route = landing::public_home_route();
    let home_label = landing::public_home_label();

    use_effect({
        let requested_path = requested_path.clone();
        move || {
            warn!(%requested_path, "rendering client not found route");
        }
    });

    rsx! {
        div { class: "min-h-screen bg-zinc-950 text-zinc-100 selection:bg-zinc-700/40",
            div { class: "grid-bg flex min-h-screen flex-col overflow-hidden",
                nav { class: "relative z-10 border-b border-zinc-800/80 bg-zinc-950/85 backdrop-blur-xl",
                    div { class: "mx-auto flex max-w-6xl items-center justify-between px-5 py-3 lg:px-8",
                        Link {
                            to: home_route.clone(),
                            class: "flex items-center gap-3",
                            div { class: "flex h-8 w-8 items-center justify-center rounded-xl border border-zinc-700 bg-zinc-100 text-zinc-950",
                                LogoIcon { class_name: "h-5 w-5" }
                            }
                            span { class: "text-sm font-semibold text-zinc-50", "CheenHub" }
                        }
                        Link {
                            to: Route::Login {},
                            class: "btn-p flex items-center rounded-xl bg-accent px-3 py-1.5 text-[12px] font-semibold text-white",
                            "Войти"
                        }
                    }
                }
                main { class: "relative flex flex-1 items-center px-5 py-12 lg:px-8",
                    div { class: "pointer-events-none absolute left-1/2 top-16 h-[520px] w-[820px] -translate-x-1/2 rounded-full bg-accent/5 blur-3xl" }
                    section { class: "relative mx-auto grid w-full max-w-6xl gap-10 lg:grid-cols-[minmax(0,1fr)_360px] lg:items-center",
                        div { class: "max-w-2xl",
                            div { class: "a1 mb-5 inline-flex items-center gap-2 rounded-full border border-zinc-800 bg-zinc-900/80 px-3.5 py-1.5 text-[11px] uppercase tracking-[0.22em] text-zinc-400",
                                span { class: "relative flex h-2 w-2",
                                    span { class: "glow-ring" }
                                    span { class: "relative h-2 w-2 rounded-full bg-accent/80" }
                                }
                                "Маршрут не найден"
                            }
                            h1 { class: "a2 text-5xl font-semibold leading-[1.05] text-zinc-50 sm:text-6xl lg:text-[80px]",
                                "404"
                            }
                            p { class: "a3 mt-5 max-w-xl text-[16px] leading-relaxed text-zinc-400",
                                "Такой страницы в CheenHub нет. Возможно, ссылка устарела или адрес набран с ошибкой."
                            }
                            p { class: "a3 mt-3 max-w-xl break-all rounded-lg border border-zinc-800 bg-zinc-900/70 px-4 py-3 text-[13px] text-zinc-500",
                                "{requested_path}"
                            }
                            div { class: "a4 mt-7 flex flex-wrap gap-3",
                                Link {
                                    to: home_route,
                                    class: "btn-p inline-flex items-center rounded-lg bg-accent px-4 py-2 text-sm font-semibold text-white",
                                    "{home_label}"
                                }
                                if has_session {
                                    Link {
                                        to: Route::AppHome {},
                                        class: "btn-g inline-flex items-center rounded-lg border border-zinc-800 bg-zinc-900 px-4 py-2 text-sm font-medium text-zinc-300",
                                        "Вернуться в приложение"
                                    }
                                }
                            }
                        }
                        aside { class: "a5 rounded-lg border border-zinc-800 bg-zinc-950/75 p-5 shadow-2xl shadow-black/30",
                            div { class: "flex items-center justify-between border-b border-zinc-800 pb-4",
                                span { class: "text-[11px] uppercase tracking-[0.22em] text-zinc-500", "Сигнал" }
                                span { class: "rounded-full bg-accent/10 px-2 py-1 text-[11px] font-medium text-blue-300", "lost" }
                            }
                            div { class: "mt-5 space-y-3",
                                div { class: "h-2 w-3/4 rounded-full bg-zinc-800" }
                                div { class: "h-2 w-1/2 rounded-full bg-zinc-800" }
                                div { class: "flex items-end gap-1 pt-4 text-accent",
                                    span { class: "bar h-5" }
                                    span { class: "bar h-9" }
                                    span { class: "bar h-6" }
                                    span { class: "bar h-11" }
                                }
                            }
                            p { class: "mt-6 text-sm leading-relaxed text-zinc-500",
                                "Вернитесь на рабочий маршрут, чтобы продолжить голосовой чат и управление серверами."
                            }
                        }
                    }
                }
            }
        }
    }
}
