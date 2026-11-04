use super::{Format, Formatted, apply};

#[test]
fn bold_selection_wraps_text_and_preserves_selection() {
    let result = apply("Привет, CheenHub!", 8..16, Format::Bold);

    assert_eq!(
        result,
        Some(Formatted {
            text: "Привет, **CheenHub**!".into(),
            cursor: 18,
            selection: 10..18
        })
    );
}

#[test]
fn supported_styles_wrap_the_same_selection() {
    for (format, expected, start) in [
        (Format::Italic, "*текст*", 1),
        (Format::Underline, "__текст__", 2),
        (Format::Strike, "~~текст~~", 2),
        (Format::Spoiler, "||текст||", 2),
        (Format::InlineCode, "`текст`", 1),
        (
            Format::Link("https://example.org".into()),
            "[текст](https://example.org)",
            1,
        ),
    ] {
        let result = apply("текст", 0..5, format).unwrap();
        assert_eq!(result.text, expected);
        assert_eq!(result.selection, start..start + 5);
    }
}

#[test]
fn empty_selection_places_caret_between_markers() {
    let result = apply("", 0..0, Format::Bold).unwrap();

    assert_eq!(result.text, "****");
    assert_eq!(result.selection, 2..2);
    assert_eq!(result.cursor, 2);
}

#[test]
fn repeating_style_removes_surrounding_markers() {
    let first = apply("привет", 0..6, Format::Bold).unwrap();
    let second = apply(&first.text, first.selection, Format::Bold).unwrap();

    assert_eq!(second.text, "привет");
    assert_eq!(second.selection, 0..6);
}

#[test]
fn selecting_entire_formatted_fragment_removes_markers() {
    let result = apply("до **текст** после", 3..12, Format::Bold).unwrap();

    assert_eq!(result.text, "до текст после");
    assert_eq!(result.selection, 3..8);
}

#[test]
fn emoji_and_multiline_selection_preserve_utf16_offsets() {
    let source = "а😀\nе\u{301}👩‍💻!";
    let end = source.encode_utf16().count() - 1;
    let result = apply(source, 1..end, Format::Spoiler).unwrap();

    assert_eq!(result.text, "а||😀\nе\u{301}👩‍💻||!");
    assert_eq!(result.selection, 3..end + 2);
}

#[test]
fn invalid_ranges_and_surrogate_halves_are_rejected() {
    for range in [2..2, 0..99, std::ops::Range { start: 3, end: 1 }] {
        assert_eq!(apply("а😀б", range, Format::Bold), None);
    }
}

#[test]
fn adding_italic_inside_bold_does_not_strip_bold_markers() {
    let result = apply("**текст**", 2..7, Format::Italic).unwrap();

    assert_eq!(result.text, "***текст***");
    assert_eq!(result.selection, 3..8);
}

#[test]
fn escaped_markers_are_not_removed() {
    let result = apply("\\**текст**", 3..8, Format::Bold).unwrap();

    assert_eq!(result.text, "\\****текст****");
}
