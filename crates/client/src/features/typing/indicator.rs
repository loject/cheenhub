//! Индикатор набора сообщения над формой чата.

use dioxus::prelude::*;

use super::realtime::TyperEntry;

/// Показывает, кто печатает сообщение в текущей комнате или диалоге.
///
/// Компонент не принимает идентификатор цели: он рендерит уже отфильтрованный
/// список, поэтому одна и та же разметка работает для комнат и личных диалогов.
///
/// При пустом списке компонент не резервирует место в макете, чтобы пустая строка
/// не сдвигала форму сообщения. Текст объявляется через `aria-live`, поэтому
/// скринридер сообщает о наборе без визуального изменения чата.
///
/// Троеточие не набирается текстом, а рисуется тремя анимированными точками:
/// текстовое «…» нельзя анимировать, а движение точек сразу передаёт, что набор
/// ещё продолжается.
#[component]
pub(crate) fn TypingIndicator(typers: Vec<TyperEntry>) -> Element {
    if typers.is_empty() {
        return rsx! {};
    }

    let label = typing_label(&typers);

    rsx! {
        div {
            class: "typing-indicator mx-auto flex w-full max-w-5xl shrink-0 items-center gap-2 px-3 pb-1.5 pt-1.5 text-[12px] leading-5 text-zinc-400 lg:px-8",
            role: "status",
            "aria-live": "polite",
            "aria-label": "{label}",
            span { class: "truncate", "{label}" }
            div { class: "typing-dots flex shrink-0 items-center gap-[3px]", "aria-hidden": "true",
                span { class: "typing-dot size-[3px] rounded-full bg-blue-300" }
                span { class: "typing-dot size-[3px] rounded-full bg-blue-300" }
                span { class: "typing-dot size-[3px] rounded-full bg-blue-300" }
            }
        }
    }
}

/// Собирает читаемый текст индикатора для одного, двух и нескольких авторов.
///
/// Текст использует привычные русские формулировки вместо перечисления всех
/// ников, чтобы длинный список не переполнял узкую форму на мобильном экране.
fn typing_label(typers: &[TyperEntry]) -> String {
    match typers {
        [] => String::new(),
        [author] => format!("{} печатает", author.author.nickname),
        [first, second] => {
            format!(
                "{} и {} печатают…",
                first.author.nickname, second.author.nickname
            )
        }
        _ => format!("{} печатают", typers_count_label(typers.len())),
    }
}

/// Склоняет существительное «участник» по числу печатающих.
///
/// Без правильной формы фраза «3 участников печатают» выглядит как ошибка в
/// интерфейсе, поэтому окончание выбирается явно.
fn typers_count_label(count: usize) -> String {
    let remainder_100 = count % 100;
    let remainder_10 = count % 10;
    let noun = if (11..=14).contains(&remainder_100) {
        "участников"
    } else if remainder_10 == 1 {
        "участник"
    } else if (2..=4).contains(&remainder_10) {
        "участника"
    } else {
        "участников"
    };
    format!("{count} {noun}")
}

#[cfg(test)]
mod tests {
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
}
