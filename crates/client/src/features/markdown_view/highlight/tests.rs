use super::highlight;

#[test]
fn supported_languages_preserve_code_and_produce_coloured_tokens() {
    for (language, code) in [
        ("rust", "fn main() { let x = 42; }\n"),
        ("javascript", "const x = 42;\n"),
        ("json", "{\"name\": 42}\n"),
    ] {
        let tokens = highlight(Some(language), code);
        assert_eq!(
            tokens
                .iter()
                .map(|token| token.text.as_str())
                .collect::<String>(),
            code
        );
        assert!(
            tokens.iter().any(|token| !token.class.is_empty()),
            "{language}: {tokens:?}"
        );
    }
}

#[test]
fn rust_javascript_and_json_tokens_have_expected_kinds() {
    for (language, source, expected, class) in [
        (
            "rust",
            "fn main() { let count = 42; }",
            "fn",
            "markdown-code-keyword",
        ),
        (
            "javascript",
            "const count = 42;",
            "const",
            "markdown-code-keyword",
        ),
        ("json", "{\"count\": 42}", "42", "markdown-code-constant"),
    ] {
        let tokens = highlight(Some(language), source);
        assert!(
            tokens
                .iter()
                .any(|token| token.text == expected && token.class == class),
            "{language}: {tokens:?}"
        );
    }
}

#[test]
fn missing_unknown_and_long_languages_preserve_plain_code() {
    for language in [None, Some("unknown")] {
        let tokens = highlight(language, "  текст\n");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].text, "  текст\n");
        assert_eq!(tokens[0].class, "");
    }
    let code = "x".repeat(40_000);
    assert_eq!(highlight(Some("rust"), &code)[0].text, code);
    assert!(highlight(Some("rust"), "").is_empty());
}

#[test]
fn requested_languages_have_highlighting() {
    for (language, code) in [
        ("rust", "fn main() {}"),
        ("python", "def hello():\n  return True\n"),
        ("javascript", "const a = true;"),
        ("typescript", "const a: string = 'x';"),
        ("json", "{\"value\": true}"),
        ("html", "<html><body>Hello</body></html>"),
        ("css", "body { color: red; }"),
        ("sql", "SELECT * FROM users;"),
        ("bash", "if true; then echo yes; fi"),
        ("c", "int main() { return 0; }"),
        ("cpp", "class Foo { public: int n; };"),
        ("java", "class Foo { public static void main() {} }"),
        ("go", "func main() { return }"),
    ] {
        let tokens = highlight(Some(language), code);
        assert!(
            tokens.iter().any(|token| !token.class.is_empty()),
            "{language}: {tokens:?}"
        );
        assert_eq!(
            tokens
                .iter()
                .map(|token| token.text.as_str())
                .collect::<String>(),
            code,
            "{language}"
        );
    }
}

#[test]
fn highlighting_large_code_keeps_complete_text() {
    let code = "let value = 42;\n".repeat(1_000);
    let start = std::time::Instant::now();
    let tokens = highlight(Some("rust"), &code);
    eprintln!(
        "rust highlight debug timing: {} bytes in {:?}, {} tokens",
        code.len(),
        start.elapsed(),
        tokens.len()
    );

    assert_eq!(
        tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<String>(),
        code
    );
    assert!(tokens.iter().any(|token| !token.class.is_empty()));
}
