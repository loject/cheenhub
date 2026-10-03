use super::*;
use cheenhub_contracts::realtime::TypingAuthor;
use uuid::Uuid;

fn author(nickname: &str) -> TyperEntry {
    TyperEntry {
        author: TypingAuthor {
            user_id: Uuid::new_v4().to_string(),
            nickname: nickname.to_owned(),
            avatar_url: None,
        },
        seen_at: web_time::Instant::now(),
    }
}

#[test]
fn label_switches_between_forms() {
    assert_eq!(typing_label(&[]), "");
    assert_eq!(typing_label(&[author("Аня")]), "Аня печатает");
    assert_eq!(
        typing_label(&[author("Аня"), author("Борис")]),
        "Аня и Борис печатают"
    );
    assert_eq!(
        typing_label(&[author("Аня"), author("Борис"), author("Вера")]),
        "3 участника печатают"
    );
    assert_eq!(
        typers_count_label(1),
        "1 участник",
        "единственный печатающий должен читаться без ошибки"
    );
    assert_eq!(typers_count_label(11), "11 участников");
    assert_eq!(typers_count_label(21), "21 участник");
}

#[test]
fn label_has_no_text_ellipsis() {
    // Троеточие рисуется анимированными точками: текстовый символ нельзя
    // анимировать, а рядом с точками он читался бы как дубль «печатает...».
    for label in [
        typing_label(&[author("Аня")]),
        typing_label(&[author("Аня"), author("Борис")]),
        typing_label(&[author("Аня"), author("Борис"), author("Вера")]),
    ] {
        assert!(
            !label.contains('…'),
            "в тексте индикатора не должно быть троеточия: {label}"
        );
    }
}
