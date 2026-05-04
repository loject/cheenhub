//! Landing page comparison section component.

use dioxus::prelude::*;

use crate::features::landing::components::check_badge::CheckBadge;
use crate::features::landing::components::eyebrow::Eyebrow;

#[component]
pub(crate) fn ComparisonSection() -> Element {
    rsx! {
        section { id: "why", class: "mx-auto max-w-6xl px-5 pb-20 lg:px-8",
            div { class: "rounded-[24px] border border-zinc-800 bg-zinc-900/90 p-6 shadow-[0_0_0_1px_rgba(255,255,255,0.02)]",
                div { class: "mb-6",
                    Eyebrow { label: "Сравнение", dark: true }
                    h2 { class: "text-xl font-semibold tracking-[-0.04em] text-zinc-50", "CheenHub и Discord" }
                    p { class: "mt-1 text-[13px] text-zinc-500", "Не атака. Просто честный список." }
                }
                div { class: "overflow-x-auto",
                    table { class: "w-full min-w-[540px]",
                        thead {
                            tr { class: "border-b border-zinc-800",
                                th { class: "w-1/3 pb-3 pr-4 text-left text-[11px] font-medium uppercase tracking-[0.2em] text-zinc-500", "Параметр" }
                                th { class: "pb-3 pr-4 text-left text-[11px] font-medium uppercase tracking-[0.2em] text-zinc-300", "CheenHub" }
                                th { class: "pb-3 text-left text-[11px] font-medium uppercase tracking-[0.2em] text-zinc-600", "Discord" }
                            }
                        }
                        tbody { class: "divide-y divide-zinc-800/50",
                            tr {
                                td { class: "py-3 pr-4 text-[13px] text-zinc-400", "Задержка голоса" }
                                td { class: "py-3 pr-4 text-[13px] text-zinc-200", "~1 мс" }
                                td { class: "py-3 text-[13px] text-zinc-600", "~60–120 мс через серверный релей" }
                            }
                            tr {
                                td { class: "py-3 pr-4 text-[13px] text-zinc-400", "Открытый код" }
                                td { class: "py-3 pr-4", CheckBadge { label: "Лицензия MIT" } }
                                td { class: "py-3 text-[13px] text-zinc-600", "Закрытый код" }
                            }
                            tr {
                                td { class: "py-3 pr-4 text-[13px] text-zinc-400", "Самостоятельный хостинг" }
                                td { class: "py-3 pr-4", CheckBadge { label: "Docker Compose" } }
                                td { class: "py-3 text-[13px] text-zinc-600", "Нет" }
                            }
                            tr {
                                td { class: "py-3 pr-4 text-[13px] text-zinc-400", "Трекинг и аналитика" }
                                td { class: "py-3 pr-4", CheckBadge { label: "Нет" } }
                                td { class: "py-3 text-[13px] text-zinc-600", "Да, обширная аналитика" }
                            }
                            tr {
                                td { class: "py-3 pr-4 text-[13px] text-zinc-400", "Потребление RAM" }
                                td { class: "py-3 pr-4 text-[13px] text-zinc-200", "< 120 MB в браузере" }
                                td { class: "py-3 text-[13px] text-zinc-600", "400 MB–1 GB+ в Electron" }
                            }
                            tr {
                                td { class: "py-3 pr-4 text-[13px] text-zinc-400", "Публичная разработка" }
                                td { class: "py-3 pr-4", CheckBadge { label: "Стримы на YouTube" } }
                                td { class: "py-3 text-[13px] text-zinc-600", "Нет" }
                            }
                        }
                    }
                }
            }
        }
    }
}
