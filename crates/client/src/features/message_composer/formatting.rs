//! Независимое от платформы изменение Markdown по выделению UTF-16.

use std::ops::Range;

/// Форматирование, применяемое к выделенному тексту.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Format {
    /// Жирный текст.
    Bold,
    /// Курсив.
    Italic,
    /// Подчёркивание.
    Underline,
    /// Зачёркивание.
    Strike,
    /// Спойлер.
    Spoiler,
    /// Встроенный код.
    InlineCode,
    /// Ссылка с заданным адресом.
    Link(String),
}

/// Результат редактирования; координаты измеряются в UTF-16 code units.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Formatted {
    /// Новый текст редактора.
    pub(super) text: String,
    /// Позиция курсора после операции.
    pub(super) cursor: usize,
    /// Диапазон выделения после операции.
    pub(super) selection: Range<usize>,
}

/// Применяет или снимает разметку, отклоняя некорректные UTF-16 границы.
pub(super) fn apply(text: &str, selection: Range<usize>, format: Format) -> Option<Formatted> {
    if selection.start > selection.end {
        return None;
    }
    let start = utf16_to_byte(text, selection.start)?;
    let end = utf16_to_byte(text, selection.end)?;
    let (open, close) = match format {
        Format::Bold => ("**", "**".to_owned()),
        Format::Italic => ("*", "*".to_owned()),
        Format::Underline => ("__", "__".to_owned()),
        Format::Strike => ("~~", "~~".to_owned()),
        Format::Spoiler => ("||", "||".to_owned()),
        Format::InlineCode => ("`", "`".to_owned()),
        Format::Link(url) => ("[", format!("]({url})")),
    };
    let open_units = open.encode_utf16().count();
    let close_units = close.encode_utf16().count();
    let selected = &text[start..end];

    if start != end
        && start >= open.len()
        && text[..start].ends_with(open)
        && text[end..].starts_with(&close)
        && delimiter_is_unambiguous(text, start - open.len(), end + close.len(), open, &close)
    {
        let mut result = text.to_owned();
        result.replace_range(end..end + close.len(), "");
        result.replace_range(start - open.len()..start, "");
        let range = selection.start - open_units..selection.end - open_units;
        return Some(Formatted {
            text: result,
            cursor: range.end,
            selection: range,
        });
    }

    if start != end
        && selected.len() >= open.len() + close.len()
        && selected.starts_with(open)
        && selected.ends_with(&close)
        && delimiter_is_unambiguous(text, start, end, open, &close)
    {
        let mut result = text.to_owned();
        result.replace_range(end - close.len()..end, "");
        result.replace_range(start..start + open.len(), "");
        let range = selection.start..selection.end - open_units - close_units;
        return Some(Formatted {
            text: result,
            cursor: range.end,
            selection: range,
        });
    }

    let mut result = String::with_capacity(text.len() + open.len() + close.len());
    result.push_str(&text[..start]);
    result.push_str(open);
    result.push_str(selected);
    result.push_str(&close);
    result.push_str(&text[end..]);
    let range = selection.start + open_units..selection.end + open_units;
    Some(Formatted {
        text: result,
        cursor: range.end,
        selection: range,
    })
}

fn utf16_to_byte(text: &str, offset: usize) -> Option<usize> {
    let mut units = 0;
    for (index, character) in text.char_indices() {
        if units == offset {
            return Some(index);
        }
        units += character.len_utf16();
        if units > offset {
            return None;
        }
    }
    (units == offset).then_some(text.len())
}

fn delimiter_is_unambiguous(text: &str, start: usize, end: usize, open: &str, close: &str) -> bool {
    let before = &text[..start];
    let after = &text[end..];
    if before
        .bytes()
        .rev()
        .take_while(|byte| *byte == b'\\')
        .count()
        % 2
        == 1
    {
        return false;
    }
    if open == close && open.bytes().all(|byte| byte == open.as_bytes()[0]) {
        let delimiter = open.as_bytes()[0];
        if before.as_bytes().last() == Some(&delimiter)
            || after.as_bytes().first() == Some(&delimiter)
        {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests;
