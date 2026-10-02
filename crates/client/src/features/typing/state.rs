//! Локальное состояние индикатора набора для одного чата.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use cheenhub_contracts::realtime::TypingAuthor;
use dioxus::prelude::*;
use futures_util::StreamExt;
use web_time::Instant;

use crate::features::realtime::{RealtimeConnectionStatus, RealtimeHandle};

use super::realtime::{
    self, TYPING_SWEEP_INTERVAL, TyperEntry, TypingIntent, TypingTarget, TypingTransport,
};
use crate::features::runtime::sleep_duration;

/// Интервал продления набора, который клиент отправляет серверу.
///
/// Должен быть заметно меньше времени жизни записи на сервере, чтобы потеря
/// одного пакета продления не погасила индикатор у других участников.
const TYPING_REFRESH_INTERVAL: Duration = Duration::from_secs(3);

/// Хук набора, передаваемый в форму сообщения одного чата.
///
/// Вызывающий чат передает целевые операции отправки, поэтому общая форма
/// остается независимой от модуля realtime.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct TypingNotifier {
    /// Сообщает чату о начале или завершении набора.
    intent: Callback<TypingIntent>,
}

impl TypingNotifier {
    /// Создает заглушку набора для тестов формы сообщения.
    ///
    /// Тесты проверяют отправку и отмену сообщения, а не работу realtime, поэтому
    /// реальный транспорт им не нужен: любое намерение просто поглощается.
    #[cfg(test)]
    pub(crate) fn new_for_tests() -> Self {
        Self {
            intent: Callback::new(|_| {}),
        }
    }

    /// Сообщает о начале или завершении набора.
    ///
    /// Вызывается формой на каждый ввод и при отправке сообщения. Повторные
    /// вызовы `Started` внутри интервала продления игнорируются, а повторный
    /// `Stopped` не отправляется, если набор уже снят.
    pub(crate) fn notify(&self, intent: TypingIntent) {
        self.intent.call(intent);
    }
}

/// Создает обратный вызов набора с троттлингом продлений.
///
/// Первое событие «печатает» уходит сразу на первом символе, поэтому собеседник
/// видит индикатор без задержки. Дальнейшие продления ограничены
/// [`TYPING_REFRESH_INTERVAL`], чтобы быстрый ввод не создавал поток сообщений.
///
/// Завершение набора отправляется сразу же: собеседник не должен ждать, пока
/// индикатор погаснет по таймауту.
pub(crate) fn use_typing_notifier(transport: TypingTransport) -> TypingNotifier {
    let state = use_hook(|| Rc::new(RefCell::new(TypingSendState::default())));

    let intent = use_callback(move |intent: TypingIntent| {
        let now = Instant::now();
        let action = state.borrow().action(intent, now);
        state.borrow_mut().apply(action, now);
        match action {
            TypingAction::Nothing => {}
            TypingAction::SendStart => {
                let transport = transport.clone();
                spawn(async move { transport.start().await });
            }
            TypingAction::SendStop => {
                let transport = transport.clone();
                spawn(async move { transport.stop().await });
            }
        }
    });

    TypingNotifier { intent }
}

/// Состояние отправки набора, которым управляет хук [`use_typing_notifier`].
///
/// Логика переходов вынесена из хука отдельно, чтобы она проверялась тестами без
/// таймеров Dioxus и без realtime: хук отвечает только за часы и за отправку.
#[derive(Debug, Default, Clone)]
struct TypingSendState {
    /// Момент последнего отправленного события начала или продления.
    last_start_at: Option<Instant>,
    /// Набор объявлен, то есть первый символ уже отправлен собеседникам.
    is_typing: bool,
}

/// Решение, которое принимает состояние набора для очередного намерения.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TypingAction {
    /// Ничего не отправлять: повторное завершение либо продление внутри
    /// интервала троттлинга.
    Nothing,
    /// Отправить начало набора или его продление.
    SendStart,
    /// Отправить завершение набора.
    SendStop,
}

impl TypingSendState {
    /// Определяет действие для намерения с учетом прошедшего с прошлого события времени.
    ///
    /// Начало уходит по «ведущему краю» — сразу на первом символе. Ожидание паузы
    /// в наборе здесь недопустимо: пока пользователь печатает подряд, пауза не
    /// наступает и индикатор не появлялся бы минутами. Троттлинг ограничивает
    /// только повторные продления, а не первое событие.
    fn action(&self, intent: TypingIntent, now: Instant) -> TypingAction {
        match intent {
            TypingIntent::Started if !self.is_typing => TypingAction::SendStart,
            TypingIntent::Started => {
                let elapsed = self
                    .last_start_at
                    .map(|last| now.saturating_duration_since(last));
                if elapsed.is_none_or(|elapsed| elapsed >= TYPING_REFRESH_INTERVAL) {
                    TypingAction::SendStart
                } else {
                    TypingAction::Nothing
                }
            }
            TypingIntent::Stopped if self.is_typing => TypingAction::SendStop,
            TypingIntent::Stopped => TypingAction::Nothing,
        }
    }

    /// Применяет действие к состоянию.
    fn apply(&mut self, action: TypingAction, now: Instant) {
        match action {
            TypingAction::Nothing => {}
            TypingAction::SendStart => {
                self.last_start_at = Some(now);
                self.is_typing = true;
            }
            TypingAction::SendStop => {
                self.is_typing = false;
                self.last_start_at = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
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
}

/// Подписывает чат на события набора и поддерживает актуальный снимок.
///
/// Снимок запрашивается при первом подключении и после каждого переподключения:
/// события набора не хранятся на сервере, поэтому без снимка индикатор был бы
/// пустым до следующего чужого события.
///
/// Параллельно с событиями идет периодическая проверка протухания. Сервер при
/// истечении своей TTL молча выбрасывает запись, не рассылая отмену, поэтому
/// потерянный `stop` или обрыв связи иначе оставляли бы индикатор висеть до
/// переподключения. Проверка гасит такие строки по [`TYPING_PEER_TTL`].
pub(crate) fn use_typing_participants(
    realtime: &RealtimeHandle,
    target: TypingTarget,
    current_user_id: String,
) -> Signal<Vec<TyperEntry>> {
    let entries = use_signal(Vec::<TyperEntry>::new);
    let mut mutable_entries = entries;

    use_hook(move || {
        let realtime = realtime.clone();
        let target = target.clone();
        let current_user_id = current_user_id.clone();

        // Проверка протухания идет отдельной задачей рядом с обработчиком
        // событий: оба работают на одном сигнале, но блокировать друг друга не
        // должны, иначе индикатор перестает появляться вовсе.
        spawn(async move { sweep_expired_typers(mutable_entries).await });

        spawn(async move {
            let mut statuses = realtime.subscribe_connection_status();
            loop {
                let Some(status) = statuses.next().await else {
                    warn!("realtime status subscription closed before typing watcher");
                    return;
                };
                if !matches!(status, RealtimeConnectionStatus::Connected(_)) {
                    continue;
                }

                refresh_typing_snapshot(&realtime, &target, mutable_entries, &current_user_id)
                    .await;

                // Подписка идет до обработки событий, иначе первое же событие о
                // наборе может прийти раньше подписки и потеряться.
                let mut events = realtime::subscribe_typing_events(&realtime);
                while let Some(event) = events.next().await {
                    if realtime::typing_event_target(&event) != target.id() {
                        continue;
                    }
                    let (author, is_typing) = realtime::typing_event_author(&event);
                    if apply_typing_participant(
                        mutable_entries,
                        author,
                        is_typing,
                        &current_user_id,
                    ) {
                        debug!(
                            target_id = target.id(),
                            nickname = %author.nickname,
                            is_typing,
                            "typing participant changed"
                        );
                    }
                }

                mutable_entries.write().clear();
                debug!(
                    target_id = target.id(),
                    "typing watcher lost realtime events"
                );
            }
        });
    });

    entries
}

/// Обновляет снимок участников, не создавая перерисовку при отсутствии изменений.
fn apply_typing_participant(
    mut entries: Signal<Vec<TyperEntry>>,
    author: &TypingAuthor,
    is_typing: bool,
    current_user_id: &str,
) -> bool {
    let mut next = entries();
    if !realtime::apply_typing_event(
        &mut next,
        author,
        is_typing,
        current_user_id,
        Instant::now(),
    ) {
        return false;
    }
    entries.set(next);
    true
}

/// Периодически убирает участников, которые перестали продлевать набор.
async fn sweep_expired_typers(mut entries: Signal<Vec<TyperEntry>>) {
    loop {
        sleep_duration(TYPING_SWEEP_INTERVAL).await;
        let mut next = entries();
        if !realtime::drop_expired_typers(&mut next, Instant::now()) {
            continue;
        }
        let expired = next.len();
        entries.set(next);
        debug!(expired_count = expired, "expired typing indicators removed");
    }
}

/// Запрашивает снимок участников и заменяет им локальное состояние.
async fn refresh_typing_snapshot(
    realtime: &RealtimeHandle,
    target: &TypingTarget,
    mut entries: Signal<Vec<TyperEntry>>,
    current_user_id: &str,
) {
    let snapshot = match target {
        TypingTarget::Room { server_id, room_id } => {
            realtime::load_room_typing_snapshot(realtime, server_id.clone(), room_id.clone())
                .await
                .map(|snapshot| snapshot.typers)
        }
        TypingTarget::DirectMessage { conversation_id } => {
            realtime::load_direct_message_typing_snapshot(realtime, conversation_id.clone())
                .await
                .map(|snapshot| snapshot.typers)
        }
    };

    match snapshot {
        Ok(loaded) => {
            let mut next = entries();
            realtime::adopt_typing_snapshot(&mut next, loaded, current_user_id, Instant::now());
            let typers_count = next.len();
            entries.set(next);
            debug!(
                target_id = target.id(),
                typers = typers_count,
                "typing snapshot loaded"
            );
        }
        Err(error) => {
            warn!(
                target_id = target.id(),
                %error,
                "typing snapshot request failed"
            );
        }
    }
}
