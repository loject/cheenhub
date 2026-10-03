//! Разбор машинных сообщений прогресса APT.

/// Возвращает процент установки из записи pmstatus, пропуская посторонний вывод.
pub(super) fn parse_percentage(line: &str) -> Option<u8> {
    let mut fields = line.strip_prefix("pmstatus:")?.split(':');
    let package = fields.next()?;
    if package.is_empty() {
        return None;
    }
    let value = fields.next()?;
    let percentage = match value.parse::<f64>() {
        Ok(value) => value,
        Err(_) => fields.next()?.parse::<f64>().ok()?,
    };
    fields.next()?;
    (percentage.is_finite() && (0.0..=100.0).contains(&percentage))
        .then(|| percentage.round() as u8)
}

#[cfg(test)]
mod tests;
