use super::*;

/// Состояние активного набора, объявленного в момент `now`.
fn typing_state(now: Instant) -> TypingSendState {
    TypingSendState {
        last_start_at: Some(now),
        is_typing: true,
    }
}

#[test]
fn first_start_is_sent_without_delay() {
    let state = TypingSendState::default();
    assert_eq!(
        state.action(TypingIntent::Started, Instant::now()),
        TypingAction::SendStart,
        "первый символ должен объявлять набор сразу, иначе индикатор не появится"
    );
}

#[test]
fn refresh_is_throttled_between_intervals() {
    let now = Instant::now();
    let state = typing_state(now);

    assert_eq!(
        state.action(TypingIntent::Started, now + Duration::from_millis(300)),
        TypingAction::Nothing,
        "быстрая печать внутри интервала не должна слать события"
    );
    assert_eq!(
        state.action(TypingIntent::Started, now + TYPING_REFRESH_INTERVAL),
        TypingAction::SendStart,
        "продление после интервала обязано уйти, иначе индикатор погаснет по TTL"
    );
}

#[test]
fn continuous_typing_sends_first_event_immediately() {
    // Регрессия: trailing-дебаунс сбрасывался на каждой клавише, поэтому при
    // непрерывном наборе первое событие не уходило минутами.
    let now = Instant::now();
    let mut state = TypingSendState::default();
    let mut sent_at_ms = Vec::new();

    for keystroke in 0..40 {
        let at = now + Duration::from_millis(keystroke * 100);
        let action = state.action(TypingIntent::Started, at);
        state.apply(action, at);
        if action != TypingAction::Nothing {
            sent_at_ms.push(keystroke * 100);
        }
    }

    assert_eq!(
        sent_at_ms.first(),
        Some(&0),
        "первый символ должен объявить набор сразу, без ожидания паузы"
    );
    assert_eq!(
        sent_at_ms.get(1),
        Some(&3_000),
        "продление должно уйти через интервал троттлинга, а не на каждой клавише"
    );
    assert_eq!(
        sent_at_ms.len(),
        2,
        "за четыре секунды непрерывного набора уходят старт и одно продление"
    );
}

#[test]
fn stop_is_sent_immediately_and_only_once() {
    let now = Instant::now();
    let mut state = typing_state(now);

    assert_eq!(
        state.action(TypingIntent::Stopped, now + Duration::from_millis(200)),
        TypingAction::SendStop,
        "завершение не должно ждать таймаута у собеседника"
    );
    state.apply(TypingAction::SendStop, now);
    assert!(!state.is_typing, "после завершения набор считается снятым");
    assert_eq!(
        state.action(TypingIntent::Stopped, now + Duration::from_secs(1)),
        TypingAction::Nothing,
        "повторное завершение без активного набора не должно слать событие"
    );
}

#[test]
fn new_typing_after_stop_is_sent_again() {
    let now = Instant::now();
    let mut state = typing_state(now);
    state.apply(TypingAction::SendStop, now);

    assert_eq!(
        state.action(TypingIntent::Started, now + Duration::from_secs(1)),
        TypingAction::SendStart,
        "после завершения следующий набор должен объявляться снова"
    );
}
