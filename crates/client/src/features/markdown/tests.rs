use super::{MessageNode, ParseError, TableAlignment, parse_message};

fn text(s: &str) -> MessageNode {
    MessageNode::Text(s.into())
}
fn paragraph(nodes: Vec<MessageNode>) -> Vec<MessageNode> {
    vec![MessageNode::Paragraph(nodes)]
}

#[test]
fn bold_text_becomes_strong_node() {
    let result = parse_message("**жирный**").unwrap();

    assert_eq!(
        result,
        vec![MessageNode::Paragraph(vec![MessageNode::Strong(vec![
            MessageNode::Text("жирный".into())
        ])])]
    );
}

#[test]
fn inline_formatting_preserves_nesting_and_unicode() {
    let cases = [
        ("*текст*", MessageNode::Emphasis(vec![text("текст")])),
        ("__текст__", MessageNode::Underline(vec![text("текст")])),
        ("~~текст~~", MessageNode::Strikethrough(vec![text("текст")])),
        (
            "||**жирный спойлер**||",
            MessageNode::Spoiler(vec![MessageNode::Strong(vec![text("жирный спойлер")])]),
        ),
        (
            "`**не жирный**`",
            MessageNode::InlineCode("**не жирный**".into()),
        ),
    ];
    for (source, node) in cases {
        assert_eq!(
            parse_message(source).unwrap(),
            paragraph(vec![node]),
            "{source}"
        );
    }
}

#[test]
fn code_blocks_keep_language_and_content() {
    assert_eq!(
        parse_message("```rust\n**код**\n```\n").unwrap(),
        vec![MessageNode::CodeBlock {
            language: Some("rust".into()),
            code: "**код**\n".into()
        }]
    );
    assert_eq!(
        parse_message("```\nтекст\n```\n").unwrap(),
        vec![MessageNode::CodeBlock {
            language: None,
            code: "текст\n".into()
        }]
    );
}

#[test]
fn headings_keep_supported_levels_and_literal_unsupported_levels() {
    for level in 1..=6 {
        let source = format!("{} текст", "#".repeat(level));
        let result = parse_message(&source).unwrap();
        if level <= 3 {
            assert_eq!(
                result,
                vec![MessageNode::Heading {
                    level: level as u8,
                    children: vec![text("текст")]
                }]
            );
        } else {
            assert_eq!(result, vec![text(&source)]);
        }
    }
}

#[test]
fn nested_quotes_are_flattened() {
    assert_eq!(
        parse_message("> > привет").unwrap(),
        vec![MessageNode::Quote(paragraph(vec![text("привет")]))]
    );
}

#[test]
fn ordered_and_unordered_lists_preserve_items() {
    for (source, ordered, start) in [("- а\n- б", false, None), ("3. а\n4. б", true, Some(3))] {
        assert_eq!(
            parse_message(source).unwrap(),
            vec![MessageNode::List {
                ordered,
                start,
                items: vec![paragraph(vec![text("а")]), paragraph(vec![text("б")])]
            }]
        );
    }
}

#[test]
fn links_keep_url_as_data() {
    assert_eq!(
        parse_message("[текст](javascript:alert)").unwrap(),
        paragraph(vec![MessageNode::Link {
            url: "javascript:alert".into(),
            children: vec![text("текст")]
        }])
    );
}

#[test]
fn table_preserves_alignment_and_cells() {
    let nodes = parse_message("| A | B | C |\n| :--- | :---: | ---: |\n| 1 | 2 | 3 |").unwrap();
    let MessageNode::Table { alignments, rows } = &nodes[0] else {
        panic!("expected table: {nodes:?}")
    };
    assert_eq!(
        alignments,
        &[
            TableAlignment::Left,
            TableAlignment::Center,
            TableAlignment::Right
        ]
    );
    assert!(rows[0].header);
    assert!(!rows[1].header);
    assert_eq!(
        rows[1].cells,
        vec![vec![text("1")], vec![text("2")], vec![text("3")]]
    );
}

#[test]
fn invalid_input_is_preserved_and_empty_input_has_no_nodes() {
    for source in [
        "||незакрыто",
        "**незакрыто",
        ":emoji:",
        "$x^2$",
        "😀 кириллица",
    ] {
        assert_eq!(
            parse_message(source).unwrap(),
            paragraph(vec![text(source)]),
            "{source}"
        );
    }
    assert_eq!(parse_message("").unwrap(), vec![]);
}

#[test]
fn long_input_returns_explicit_error() {
    assert_eq!(
        parse_message(&"я".repeat(40_000)),
        Err(ParseError::InputTooLarge)
    );
}

#[test]
fn excessive_nesting_returns_explicit_error() {
    let input = format!("{}x{}", "> ".repeat(100), "\n");
    assert_eq!(parse_message(&input), Err(ParseError::TooComplex));
}

#[test]
fn html_images_and_checkboxes_do_not_become_active_content() {
    assert_eq!(
        parse_message("<script>alert(1)</script>").unwrap(),
        vec![text("<script>alert(1)</script>")]
    );
    assert_eq!(
        parse_message("![описание](https://example.com/a.png)").unwrap(),
        paragraph(vec![text("![описание](https://example.com/a.png)")])
    );
    assert_eq!(
        parse_message("- [x] задача").unwrap(),
        vec![MessageNode::List {
            ordered: false,
            start: None,
            items: vec![paragraph(vec![text("[x] задача")])]
        }]
    );
}

#[test]
fn escaped_markers_and_line_breaks_remain_text() {
    let nodes = parse_message("\\**не жирный**\nстрока").unwrap();
    assert!(!format!("{nodes:?}").contains("Strong"));
    let MessageNode::Paragraph(children) = &nodes[0] else {
        panic!("expected paragraph")
    };
    let plain: String = children
        .iter()
        .map(|node| match node {
            MessageNode::Text(value) => value.as_str(),
            _ => panic!("expected text: {children:?}"),
        })
        .collect();
    assert_eq!(plain, "**не жирный**\nстрока");
}

#[test]
fn triple_asterisks_combine_bold_and_italic() {
    assert_eq!(
        parse_message("***жирный курсив***").unwrap(),
        paragraph(vec![MessageNode::Emphasis(vec![MessageNode::Strong(
            vec![text("жирный курсив")]
        )])])
    );
}

#[test]
fn too_many_inline_nodes_are_rejected() {
    let input = "*a* ".repeat(5_000);
    assert_eq!(parse_message(&input), Err(ParseError::TooComplex));
}

#[test]
fn code_keeps_nonstandard_delimiters_and_line_breaks() {
    let result = parse_message("`__a__ ||b||`\n\n```\n__a__ ||b||\n```\n").unwrap();
    assert_eq!(
        result,
        vec![
            MessageNode::Paragraph(vec![MessageNode::InlineCode("__a__ ||b||".into())]),
            MessageNode::CodeBlock {
                language: None,
                code: "__a__ ||b||\n".into()
            },
        ]
    );
}

#[test]
fn blocks_and_newlines_are_preserved() {
    let result = parse_message("первая\nвторая\n\nтретья").unwrap();
    assert_eq!(
        result,
        vec![
            MessageNode::Paragraph(vec![text("первая"), text("\n"), text("вторая")]),
            MessageNode::Paragraph(vec![text("третья")])
        ]
    );
}

#[test]
fn escaped_spoiler_and_underline_delimiters_remain_literal() {
    for source in [r"\||secret||", r"\__underline__"] {
        let nodes = parse_message(source).unwrap();
        assert!(
            !format!("{nodes:?}").contains("Spoiler"),
            "{source}: {nodes:?}"
        );
        assert!(
            !format!("{nodes:?}").contains("Underline"),
            "{source}: {nodes:?}"
        );
    }
}

#[test]
fn unfinished_image_and_unknown_markup_remain_literal() {
    for source in ["![alt](unfinished", "<unclosed", "^^custom^^"] {
        assert_eq!(
            parse_message(source).unwrap(),
            paragraph(vec![text(source)]),
            "{source}"
        );
    }
}
