//! Форматирование дат и времени сообщений текстового чата.

use chrono::{DateTime, Datelike, Duration, Timelike, Utc};

const MONTH_NAMES: [&str; 12] = [
    "января",
    "февраля",
    "марта",
    "апреля",
    "мая",
    "июня",
    "июля",
    "августа",
    "сентября",
    "октября",
    "ноября",
    "декабря",
];

/// Возвращает ключ календарного дня для группировки сообщений.
pub(crate) fn message_day_key(created_at: &str) -> String {
    parse_timestamp(created_at)
        .map(|timestamp| timestamp.format("%F").to_string())
        .unwrap_or_else(|| {
            created_at
                .split('T')
                .next()
                .unwrap_or(created_at)
                .to_owned()
        })
}

/// Возвращает дружелюбную подпись дня сообщения.
pub(crate) fn friendly_message_date(created_at: &str) -> String {
    let Some(timestamp) = parse_timestamp(created_at) else {
        return message_day_key(created_at);
    };
    let message_date = timestamp.date_naive();
    let today = Utc::now().date_naive();
    let age = today.signed_duration_since(message_date);

    if age == Duration::zero() {
        "Сегодня".to_owned()
    } else if age == Duration::days(1) {
        "Вчера".to_owned()
    } else if age > Duration::days(365) {
        format!(
            "{} {} {}",
            message_date.day(),
            month_name(message_date.month0()),
            message_date.year()
        )
    } else {
        format!(
            "{} {}",
            message_date.day(),
            month_name(message_date.month0())
        )
    }
}

/// Возвращает полную дату и время для подсказки сообщения.
pub(super) fn full_message_datetime(created_at: &str) -> String {
    parse_timestamp(created_at)
        .map(|timestamp| {
            let date = timestamp.date_naive();
            format!(
                "{} {} {} г., {:02}:{:02}:{:02}",
                date.day(),
                month_name(date.month0()),
                date.year(),
                timestamp.hour(),
                timestamp.minute(),
                timestamp.second()
            )
        })
        .unwrap_or_else(|| created_at.to_owned())
}

fn parse_timestamp(created_at: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(created_at)
        .ok()
        .map(|timestamp| timestamp.with_timezone(&Utc))
}

fn month_name(month_index: u32) -> &'static str {
    MONTH_NAMES.get(month_index as usize).copied().unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::{friendly_message_date, full_message_datetime, message_day_key};

    #[test]
    fn formats_full_datetime_for_tooltip() {
        assert_eq!(
            full_message_datetime("2025-07-12T08:09:10+00:00"),
            "12 июля 2025 г., 08:09:10"
        );
    }

    #[test]
    fn uses_full_date_for_messages_older_than_year() {
        assert_eq!(
            friendly_message_date("2020-07-12T08:09:10+00:00"),
            "12 июля 2020"
        );
    }

    #[test]
    fn groups_equivalent_timestamps_by_utc_day() {
        assert_eq!(
            message_day_key("2025-07-12T23:30:00-02:00"),
            message_day_key("2025-07-13T01:30:00+00:00")
        );
    }
}
