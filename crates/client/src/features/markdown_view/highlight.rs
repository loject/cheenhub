//! Преобразование исходного кода в безопасные текстовые фрагменты.

use std::cell::RefCell;
use syntect::{
    easy::ScopeRegionIterator,
    parsing::{ParseState, Scope, ScopeStack, SyntaxSet},
};

thread_local! { static SYNTAXES: RefCell<Option<SyntaxSet>> = const { RefCell::new(None) }; }

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Token {
    pub(super) text: String,
    pub(super) class: &'static str,
}

/// Выделяет синтаксические фрагменты, сохраняя исходный текст.
pub(super) fn highlight(language: Option<&str>, code: &str) -> Vec<Token> {
    let plain = || {
        vec![Token {
            text: code.to_owned(),
            class: "",
        }]
    };
    let Some(language) = language else {
        return plain();
    };
    if code.len() > 32_768 {
        dioxus::logger::tracing::debug!(
            bytes = code.len(),
            "code block exceeds highlighting limit"
        );
        return plain();
    }
    SYNTAXES.with_borrow_mut(|slot| {
        let syntaxes = slot.get_or_insert_with(two_face::syntax::extra_newlines);
        let name = match language.to_ascii_lowercase().as_str() {
            "rust" | "rs" => "Rust",
            "python" | "py" => "Python",
            "javascript" | "js" => "JavaScript",
            "typescript" | "ts" => "TypeScript",
            "json" => "JSON",
            "html" => "HTML",
            "css" => "CSS",
            "sql" => "SQL",
            "bash" | "sh" | "shell" => "Bourne Again Shell (bash)",
            "c" => "C",
            "cpp" | "c++" => "C++",
            "java" => "Java",
            "go" => "Go",
            _ => return plain(),
        };
        let Some(syntax) = syntaxes.find_syntax_by_name(name) else {
            return plain();
        };
        let mut state = ParseState::new(syntax);
        let mut stack = ScopeStack::new();
        let mut tokens = Vec::new();
        let categories = [
            ("comment", "markdown-code-comment"),
            ("string", "markdown-code-string"),
            ("keyword", "markdown-code-keyword"),
            ("storage", "markdown-code-keyword"),
            ("constant", "markdown-code-constant"),
            ("entity.name", "markdown-code-name"),
            ("support", "markdown-code-name"),
        ].into_iter().filter_map(|(name, class)| Scope::new(name).ok().map(|scope| (scope, class))).collect::<Vec<_>>();
        for line in code.split_inclusive('\n') {
            let ops = match state.parse_line(line, syntaxes) {
                Ok(ops) => ops,
                Err(error) => {
                    dioxus::logger::tracing::warn!(language, %error, "code block highlighting failed");
                    return plain();
                }
            };
            for (text, op) in ScopeRegionIterator::new(&ops, line) {
                if stack.apply(op).is_err() {
                    dioxus::logger::tracing::warn!(language, "code block highlighting scope failed");
                    return plain();
                }
                if text.is_empty() {
                    continue;
                }
                let class = stack
                    .scopes
                    .iter()
                    .rev()
                    .find_map(|scope| categories.iter().find_map(|(prefix, class)| prefix.is_prefix_of(*scope).then_some(*class)))
                    .unwrap_or("");
                tokens.push(Token {
                    text: text.to_owned(),
                    class,
                });
            }
        }
        tokens
    })
}

#[cfg(test)]
mod tests;
