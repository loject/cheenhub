//! Просмотр и ручной экспорт локального журнала настольного клиента.

use chrono::{Local, NaiveDateTime, TimeZone};
use dioxus::prelude::*;

use crate::features::clipboard::copy_text;
use crate::features::diagnostics::{self, ExportLimits, LogEntry, LogFilter};

const PAGE_SIZE: usize = 100;

#[derive(Clone)]
struct LogRequest {
    filter: LogFilter,
    offset: usize,
    sequence: u64,
}

fn datetime_bound(value: &str) -> Result<Option<String>, String> {
    if value.trim().is_empty() {
        return Ok(None);
    }
    let date = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M")
        .map_err(|_| "Некорректная дата или время в фильтре.".to_owned())?;
    Local
        .from_local_datetime(&date)
        .single()
        .map(|date| Some(date.to_rfc3339()))
        .ok_or_else(|| "Указанное местное время неоднозначно или не существует.".to_owned())
}

fn format_local_timestamp(value: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|date| {
            date.with_timezone(&Local)
                .format("%Y-%m-%d %H:%M:%S %:z")
                .to_string()
        })
        .unwrap_or_else(|_| value.to_owned())
}

fn inclusive_end_bound(value: &str) -> Result<Option<String>, String> {
    datetime_bound(value)?
        .map(|bound| {
            let start =
                chrono::DateTime::parse_from_rfc3339(&bound).map_err(|error| error.to_string())?;
            let end = start + chrono::Duration::minutes(1) - chrono::Duration::nanoseconds(1);
            Ok(end.to_rfc3339_opts(chrono::SecondsFormat::Nanos, false))
        })
        .transpose()
}

fn filter_from_input(
    level: &str,
    target: &str,
    text: &str,
    since: &str,
    until: &str,
) -> Result<LogFilter, String> {
    let since = datetime_bound(since)?;
    let until = inclusive_end_bound(until)?;
    if let (Some(start), Some(end)) = (&since, &until) {
        let start = chrono::DateTime::parse_from_rfc3339(start).map_err(|e| e.to_string())?;
        let end = chrono::DateTime::parse_from_rfc3339(end).map_err(|e| e.to_string())?;
        if start > end {
            return Err("Начало периода не может быть позже окончания.".to_owned());
        }
    }
    Ok(LogFilter {
        level: (!level.is_empty()).then(|| level.to_owned()),
        target: target.trim().to_owned(),
        text: text.trim().to_owned(),
        since,
        until,
    })
}

#[component]
pub(crate) fn DiagnosticsSettingsSection() -> Element {
    let mut level = use_signal(String::new);
    let mut target = use_signal(String::new);
    let mut search = use_signal(String::new);
    let mut since = use_signal(String::new);
    let mut until = use_signal(String::new);
    let mut request = use_signal(|| LogRequest {
        filter: LogFilter::default(),
        offset: 0,
        sequence: 0,
    });
    let mut entries = use_signal(Vec::<LogEntry>::new);
    let mut total = use_signal(|| 0_usize);
    let mut loading = use_signal(|| false);
    let mut exporting = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut notice = use_signal(String::new);

    // Изменение запроса запускает чтение очередной страницы в blocking pool.
    use_effect(move || {
        let current = request();
        let sequence = current.sequence;
        let offset = current.offset;
        let filter = current.filter;
        loading.set(true);
        error.set(None);
        spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                diagnostics::list_logs(&filter, offset, PAGE_SIZE)
            })
            .await;
            if request.peek().sequence != sequence {
                return;
            }
            match result {
                Ok(Ok(page)) => {
                    total.set(page.total);
                    if offset == 0 {
                        entries.set(page.entries);
                    } else {
                        entries.write().extend(page.entries);
                    }
                }
                Ok(Err(err)) => error.set(Some(format!("Не удалось прочитать журнал: {err}"))),
                Err(err) => error.set(Some(format!("Не удалось завершить чтение: {err}"))),
            }
            loading.set(false);
        });
    });

    let displayed = entries();
    let total_count = total();
    let is_loading = loading();
    let is_exporting = exporting();
    let can_load_more = !is_loading && displayed.len() < total_count;

    rsx! {
        div { class: "space-y-4",
            div { class: "rounded-2xl border border-zinc-800 bg-zinc-950/70 p-4",
                div { class: "flex flex-wrap items-start justify-between gap-3",
                    div {
                        h3 { class: "text-[15px] font-semibold text-zinc-100", "Журнал событий" }
                        p { class: "mt-1 text-[12px] leading-5 text-zinc-500", "Логи хранятся только на этом компьютере. Экспорт выполняется вручную." }
                    }
                    div { class: "flex flex-wrap gap-2",
                        button {
                            r#type: "button",
                            disabled: is_loading,
                            class: "rounded-xl border border-zinc-700 bg-zinc-900 px-3 py-2 text-[12px] text-zinc-200 transition hover:bg-zinc-800 disabled:opacity-50",
                            onclick: move |_| {
                                entries.set(Vec::new());
                                total.set(0);
                                let previous = request.peek().clone();
                                request.set(LogRequest { offset: 0, sequence: previous.sequence + 1, ..previous });
                            },
                            "Обновить"
                        }
                        button {
                            r#type: "button",
                            disabled: is_exporting,
                            class: "rounded-xl border border-accent/30 bg-accent/10 px-3 py-2 text-[12px] font-medium text-blue-100 transition hover:bg-accent/20 disabled:opacity-50",
                            onclick: move |_| {
                                if exporting() { return; }
                                exporting.set(true);
                                notice.set(String::new());
                                let filter = request.peek().filter.clone();
                                spawn(async move {
                                    let chosen = tokio::task::spawn_blocking(|| {
                                        rfd::FileDialog::new()
                                            .add_filter("JSON Lines", &["jsonl"])
                                            .set_file_name("cheenhub-logs.jsonl")
                                            .save_file()
                                    }).await;
                                    match chosen {
                                        Ok(Some(path)) => {
                                            let result = tokio::task::spawn_blocking(move || {
                                                diagnostics::export_logs(&path, &filter, ExportLimits::default())
                                            }).await;
                                            match result {
                                                Ok(Ok(exported)) => {
                                                    error.set(None);
                                                    let limit_note = if exported.truncated { " (достигнут лимит экспорта)" } else { "" };
                                                    notice.set(format!("Экспортировано {} записей, {} байт{}.", exported.lines, exported.bytes, limit_note));
                                                }
                                                Ok(Err(err)) => error.set(Some(format!("Ошибка экспорта: {err}"))),
                                                Err(err) => error.set(Some(format!("Ошибка фонового экспорта: {err}"))),
                                            }
                                        }
                                        Ok(None) => {}
                                        Err(err) => error.set(Some(format!("Не удалось открыть диалог сохранения: {err}"))),
                                    }
                                    exporting.set(false);
                                });
                            },
                            if is_exporting { "Экспортируем..." } else { "Экспорт JSONL" }
                        }
                    }
                }
                div { class: "mt-4 grid grid-cols-1 gap-3 sm:grid-cols-2",
                    label { class: "space-y-1 text-[11px] text-zinc-400",
                        "Уровень"
                        select {
                            class: "w-full rounded-lg border border-zinc-700 bg-zinc-900 p-2 text-[12px] text-zinc-100",
                            value: "{level()}",
                            onchange: move |event| level.set(event.value()),
                            option { value: "", "Все уровни" }
                            option { value: "ERROR", "ERROR" }
                            option { value: "WARN", "WARN" }
                            option { value: "INFO", "INFO" }
                            option { value: "DEBUG", "DEBUG" }
                            option { value: "TRACE", "TRACE" }
                        }
                    }
                    label { class: "space-y-1 text-[11px] text-zinc-400",
                        "Модуль (target)"
                        input {
                            r#type: "text",
                            class: "w-full rounded-lg border border-zinc-700 bg-zinc-900 p-2 text-[12px] text-zinc-100",
                            placeholder: "voice_chat",
                            value: "{target()}",
                            oninput: move |event| target.set(event.value()),
                        }
                    }
                    label { class: "space-y-1 text-[11px] text-zinc-400",
                        "Поиск по записи"
                        input {
                            r#type: "text",
                            class: "w-full rounded-lg border border-zinc-700 bg-zinc-900 p-2 text-[12px] text-zinc-100",
                            placeholder: "Ошибка, connection, session...",
                            value: "{search()}",
                            oninput: move |event| search.set(event.value()),
                        }
                    }
                    div { class: "flex items-end",
                        button {
                            r#type: "button",
                            disabled: is_loading,
                            class: "w-full rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-2 text-[12px] font-medium text-zinc-100 transition hover:bg-zinc-800 disabled:opacity-50",
                            onclick: move |_| {
                                match filter_from_input(&level(), &target(), &search(), &since(), &until()) {
                                    Ok(filter) => {
                                        entries.set(Vec::new());
                                        total.set(0);
                                        notice.set(String::new());
                                        let previous = request.peek().clone();
                                        request.set(LogRequest { filter, offset: 0, sequence: previous.sequence + 1 });
                                    }
                                    Err(message) => error.set(Some(message)),
                                }
                            },
                            "Применить фильтры"
                        }
                    }
                    label { class: "space-y-1 text-[11px] text-zinc-400",
                        "С"
                        input {
                            r#type: "datetime-local",
                            class: "w-full rounded-lg border border-zinc-700 bg-zinc-900 p-2 text-[12px] text-zinc-100",
                            value: "{since()}",
                            oninput: move |event| since.set(event.value()),
                        }
                    }
                    label { class: "space-y-1 text-[11px] text-zinc-400",
                        "По"
                        input {
                            r#type: "datetime-local",
                            class: "w-full rounded-lg border border-zinc-700 bg-zinc-900 p-2 text-[12px] text-zinc-100",
                            value: "{until()}",
                            oninput: move |event| until.set(event.value()),
                        }
                    }
                }
                p { class: "mt-3 text-[11px] text-zinc-500", "Экспорт использует применённые фильтры; максимум 50 000 записей или 10 МиБ." }
            }
            if let Some(message) = error() {
                div { class: "rounded-xl border border-red-500/30 bg-red-500/10 p-3 text-[12px] text-red-200", "{message}" }
            }
            if !notice().is_empty() {
                div { class: "rounded-xl border border-emerald-500/25 bg-emerald-500/10 p-3 text-[12px] text-emerald-200", "{notice()}" }
            }
            div { class: "rounded-2xl border border-zinc-800 bg-zinc-950/70 p-3",
                div { class: "flex items-center justify-between gap-3 px-1 pb-3",
                    p { class: "text-[12px] text-zinc-400", "Показано {displayed.len()} из {total_count} · время местное" }
                    if is_loading { span { class: "text-[11px] text-zinc-500", "Загрузка..." } }
                }
                if displayed.is_empty() && !is_loading {
                    p { class: "rounded-xl bg-zinc-900/50 p-5 text-center text-[12px] text-zinc-500", "По выбранным фильтрам событий нет." }
                }
                div { class: "space-y-2",
                    for entry in displayed {
                        div { class: "rounded-xl border border-zinc-800/80 bg-zinc-900/45 p-3",
                            div { class: "flex flex-wrap items-center gap-2",
                                span { class: "font-mono text-[11px] text-zinc-500", "{format_local_timestamp(&entry.timestamp)}" }
                                span { class: level_class(&entry.level), "{entry.level}" }
                                span { class: "min-w-0 break-all font-mono text-[11px] text-blue-200/75", "{entry.target}" }
                                button {
                                    r#type: "button",
                                    class: "ml-auto rounded-lg border border-zinc-700 px-2 py-1 text-[11px] text-zinc-300 hover:bg-zinc-800",
                                    onclick: move |_| {
                                        let raw = entry.raw.clone();
                                        spawn(async move {
                                            match copy_text(raw).await {
                                                Ok(()) => notice.set("JSON-запись скопирована в буфер обмена.".to_owned()),
                                                Err(err) => error.set(Some(err)),
                                            }
                                        });
                                    },
                                    "Копировать JSON"
                                }
                            }
                            p { class: "mt-2 max-h-56 overflow-y-auto whitespace-pre-wrap break-words text-[12px] leading-5 text-zinc-200", "{entry.message}" }
                        }
                    }
                }
                if can_load_more {
                    button {
                        r#type: "button",
                        class: "mt-3 w-full rounded-xl border border-zinc-800 bg-zinc-900 px-3 py-2 text-[12px] text-zinc-200 transition hover:bg-zinc-800",
                        onclick: move |_| {
                            let previous = request.peek().clone();
                            request.set(LogRequest { offset: entries.peek().len(), sequence: previous.sequence + 1, ..previous });
                        },
                        "Загрузить ещё 100"
                    }
                }
            }
        }
    }
}

fn level_class(level: &str) -> &'static str {
    match level {
        "ERROR" => "rounded bg-red-500/15 px-2 py-0.5 font-mono text-[10px] text-red-300",
        "WARN" => "rounded bg-amber-500/10 px-2 py-0.5 font-mono text-[10px] text-amber-300",
        "INFO" => "rounded bg-blue-500/15 px-2 py-0.5 font-mono text-[10px] text-blue-300",
        _ => "rounded bg-zinc-700 px-2 py-0.5 font-mono text-[10px] text-zinc-300",
    }
}

#[cfg(test)]
#[path = "diagnostics_section/tests.rs"]
mod tests;
