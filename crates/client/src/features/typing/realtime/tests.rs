use super::*;

fn entry(nickname: &str, seen_at: Instant) -> TyperEntry {
    TyperEntry {
        author: TypingAuthor {
            user_id: nickname.to_owned(),
            nickname: nickname.to_owned(),
            avatar_url: None,
        },
        seen_at,
    }
}

#[test]
fn lost_stop_event_still_expires_the_indicator() {
    // Регрессия: потерянный stop или сбой вкладки оставлял индикатор висеть до
    // переподключения realtime. Теперь строка гаснет по локальному TTL.
    let mut typers = vec![entry("Аня", Instant::now())];

    assert!(
        !drop_expired_typers(&mut typers, Instant::now() + TYPING_PEER_TTL / 2),
        "до истечения TTL строка должна оставаться"
    );
    assert!(
        drop_expired_typers(&mut typers, Instant::now() + TYPING_PEER_TTL),
        "после истечения TTL строка обязана исчезнуть"
    );
    assert!(typers.is_empty());
}

#[test]
fn refresh_extends_indicator_lifetime() {
    let author = TypingAuthor {
        user_id: "Аня".to_owned(),
        nickname: "Аня".to_owned(),
        avatar_url: None,
    };
    let mut typers = vec![entry("Аня", Instant::now())];

    for step in 1..=3 {
        apply_typing_event(
            &mut typers,
            &author,
            true,
            "me",
            Instant::now() + TYPING_PEER_TTL / 2 * step,
        );
    }

    assert!(
        !drop_expired_typers(&mut typers, Instant::now() + TYPING_PEER_TTL * 2),
        "продления должны сдвигать момент протухания"
    );
    assert!(
        drop_expired_typers(&mut typers, Instant::now() + TYPING_PEER_TTL * 4),
        "без новых продлений строка в итоге протухает"
    );
}

#[test]
fn stop_event_removes_immediately_without_waiting_for_ttl() {
    let author = TypingAuthor {
        user_id: "Аня".to_owned(),
        nickname: "Аня".to_owned(),
        avatar_url: None,
    };
    let mut typers = vec![entry("Аня", Instant::now())];

    assert!(apply_typing_event(
        &mut typers,
        &author,
        false,
        "me",
        Instant::now(),
    ));
    assert!(
        typers.is_empty(),
        "явная отмена должна гасить индикатор сразу"
    );
}

#[test]
fn snapshot_resets_lifetime() {
    let author = TypingAuthor {
        user_id: "Аня".to_owned(),
        nickname: "Аня".to_owned(),
        avatar_url: None,
    };
    let mut typers = vec![entry("Аня", Instant::now())];

    adopt_typing_snapshot(&mut typers, vec![author], "me", Instant::now());

    assert!(
        !drop_expired_typers(&mut typers, Instant::now() + TYPING_PEER_TTL / 2),
        "снимок сервера должен сбрасывать время жизни строки"
    );
}
