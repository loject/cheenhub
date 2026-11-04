//! Разбор Markdown сообщений без HTML и зависимости модели от парсера.

mod autolink;
mod parser;

pub use parser::{ParseError, parse_message};

/// Узел содержимого сообщения для рекурсивного отображения.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MessageNode {
    /// Обычный текст, включая переносы строк.
    Text(String),
    /// Граница абзаца.
    Paragraph(Vec<MessageNode>),
    /// Жирное начертание.
    Strong(Vec<MessageNode>),
    /// Курсив.
    Emphasis(Vec<MessageNode>),
    /// Подчёркивание.
    Underline(Vec<MessageNode>),
    /// Зачёркивание.
    Strikethrough(Vec<MessageNode>),
    /// Скрываемый текст.
    Spoiler(Vec<MessageNode>),
    /// Содержимое встроенного кода.
    InlineCode(String),
    /// Блок кода и необязательное имя языка.
    CodeBlock {
        /// Первое слово строки языка, если оно задано.
        language: Option<String>,
        /// Содержимое блока с переносами строк.
        code: String,
    },
    /// Заголовок уровня от одного до трёх.
    Heading {
        /// Уровень заголовка.
        level: u8,
        /// Содержимое заголовка.
        children: Vec<MessageNode>,
    },
    /// Цитата без вложенных уровней.
    Quote(Vec<MessageNode>),
    /// Маркированный или нумерованный список.
    List {
        /// Признак нумерованного списка.
        ordered: bool,
        /// Начальный номер нумерованного списка.
        start: Option<u64>,
        /// Пункты с сохранёнными границами блоков.
        items: Vec<Vec<MessageNode>>,
    },
    /// Ссылка; URL хранится как данные и не открывается парсером.
    Link {
        /// Адрес ссылки.
        url: String,
        /// Подпись ссылки.
        children: Vec<MessageNode>,
    },
    /// Таблица с выравниванием и строками.
    Table {
        /// Выравнивание столбцов по порядку.
        alignments: Vec<TableAlignment>,
        /// Строкы и их ячейки.
        rows: Vec<TableRow>,
    },
}

/// Выравнивание столбца таблицы.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableAlignment {
    /// Выравнивание не задано.
    None,
    /// По левому краю.
    Left,
    /// По центру.
    Center,
    /// По правому краю.
    Right,
}

/// Строка таблицы с признаком заголовка.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableRow {
    /// Является ли строка заголовком.
    pub header: bool,
    /// Содержимое ячеек по порядку.
    pub cells: Vec<Vec<MessageNode>>,
}

#[cfg(test)]
mod tests;
