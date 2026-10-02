//! Состояния загрузки системных метрик хоста.
//!
//! Заглушки относятся только к отдельным блокам: страница дашборда и блок
//! активности голосового чата загружаются независимо от них.

use dioxus::prelude::*;

/// Показывает скелет системных метрик до первого ответа сервера.
pub(super) fn metrics_loader() -> Element {
    rsx! {
        div { class: "mt-6 grid gap-5",
            div { class: "h-72 animate-pulse rounded-[20px] border border-zinc-800 bg-zinc-950/70" }
            div { class: "grid gap-5 lg:grid-cols-2",
                div { class: "h-64 animate-pulse rounded-[20px] border border-zinc-800 bg-zinc-950/60" }
                div { class: "h-64 animate-pulse rounded-[20px] border border-zinc-800 bg-zinc-950/60" }
            }
            div { class: "mt-5 h-40 animate-pulse rounded-[20px] border border-zinc-800 bg-zinc-950/60" }
        }
    }
}

/// Сообщает о недоступности системных метрик и позволяет повторить запрос.
pub(super) fn metrics_error(message: String, retry: EventHandler<()>) -> Element {
    rsx! {
        div { class: "mt-6 rounded-[20px] border border-red-500/20 bg-red-500/10 p-5",
            h2 { class: "text-lg font-semibold text-red-100", "Не удалось загрузить показатели нагрузки" }
            p { class: "mt-2 text-pretty text-sm leading-6 text-red-100/75", "{message}" }
            p { class: "mt-2 text-pretty text-[13px] leading-5 text-zinc-400", "Активность голосового чата выше обновляется независимо от системных метрик." }
            button {
                r#type: "button",
                class: "mt-4 inline-flex min-h-11 items-center justify-center rounded-xl bg-red-400 px-4 text-[13px] font-semibold text-zinc-950 transition-[background-color,scale] duration-150 hover:bg-red-300 active:scale-[0.96]",
                onclick: move |_| retry(()),
                "Повторить"
            }
        }
    }
}

/// Предупреждает, что показания показываются из последних доступных измерений.
pub(super) fn stale_metrics_notice() -> Element {
    rsx! {
        div { class: "mt-6 rounded-2xl bg-amber-400/10 px-4 py-3 text-[13px] text-amber-100 shadow-[0_0_0_1px_rgba(251,191,36,0.2)]", role: "status",
            "Показываем последние доступные данные. Новые измерения временно не поступают."
        }
    }
}

/// Пустое состояние отдельной секции метрик.
pub(super) fn section_without_data(message: &'static str) -> Element {
    rsx! {
        div { class: "mt-3 rounded-2xl border border-dashed border-zinc-800 bg-zinc-950/40 px-4 py-6 text-center",
            p { class: "text-[13px] leading-5 text-zinc-500", "{message}" }
        }
    }
}
