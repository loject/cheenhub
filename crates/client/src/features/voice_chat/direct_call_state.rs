//! Клиентское состояние lifecycle личного звонка.

use cheenhub_contracts::realtime::{
    DirectCallLifecycleEvent, DirectCallResponse, DirectCallSnapshot, DirectCallState,
};
use dioxus::prelude::*;

use crate::features::realtime::RealtimeHandle;

use super::direct_call_realtime;
use super::state::{VoiceConnectionHandle, VoiceRoomTarget, VoiceRoomTargetKind};

const ENDED_STATE_VISIBLE_MS: u32 = 2_400;

/// Состояние интерфейса личного звонка.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DirectCallUiState {
    /// Личного звонка сейчас нет.
    Idle,
    /// Запрос на начало звонка отправляется.
    Starting {
        /// Цель исходящего звонка.
        target: VoiceRoomTarget,
    },
    /// Backend вернул актуальный снимок звонка.
    Call(DirectCallSnapshot),
    /// Последнее действие завершилось ошибкой.
    Error {
        /// Цель, для которой не удалось начать звонок.
        target: Option<VoiceRoomTarget>,
        /// Снимок звонка, если ошибка возникла после его создания.
        call: Option<DirectCallSnapshot>,
        /// Сообщение для пользователя.
        message: String,
    },
}

/// Контекст управления signaling личного звонка.
#[derive(Clone)]
pub(crate) struct DirectCallHandle {
    state: Signal<DirectCallUiState>,
    busy: Signal<bool>,
    realtime: RealtimeHandle,
    voice: VoiceConnectionHandle,
    current_user_id: String,
}

impl DirectCallHandle {
    /// Создаёт контекст signaling личного звонка.
    pub(super) fn new(
        state: Signal<DirectCallUiState>,
        busy: Signal<bool>,
        realtime: RealtimeHandle,
        voice: VoiceConnectionHandle,
        current_user_id: String,
    ) -> Self {
        Self {
            state,
            busy,
            realtime,
            voice,
            current_user_id,
        }
    }

    /// Возвращает текущее состояние интерфейса.
    pub(crate) fn state(&self) -> DirectCallUiState {
        (self.state)()
    }

    /// Сообщает, выполняется ли signaling-действие.
    pub(crate) fn busy(&self) -> bool {
        (self.busy)()
    }

    /// Возвращает звонок выбранного личного диалога.
    pub(crate) fn call_for_conversation(
        &self,
        conversation_id: &str,
    ) -> Option<DirectCallSnapshot> {
        match self.state() {
            DirectCallUiState::Call(call) if call.conversation_id == conversation_id => Some(call),
            DirectCallUiState::Error {
                call: Some(call), ..
            } if call.conversation_id == conversation_id => Some(call),
            _ => None,
        }
    }

    /// Сообщает, начинается ли звонок в выбранном личном диалоге.
    pub(crate) fn is_starting_conversation(&self, conversation_id: &str) -> bool {
        matches!(
            self.state(),
            DirectCallUiState::Starting { target } if target.room_id == conversation_id
        )
    }

    /// Сообщает, нужно ли показывать интерфейс звонка выбранного личного диалога.
    pub(crate) fn is_visible_for_conversation(&self, conversation_id: &str) -> bool {
        match self.state() {
            DirectCallUiState::Starting { target } => target.room_id == conversation_id,
            DirectCallUiState::Call(call) => call.conversation_id == conversation_id,
            DirectCallUiState::Error { target, call, .. } => {
                target
                    .as_ref()
                    .is_some_and(|target| target.room_id == conversation_id)
                    || call
                        .as_ref()
                        .is_some_and(|call| call.conversation_id == conversation_id)
            }
            DirectCallUiState::Idle => false,
        }
    }

    /// Возвращает ошибку, относящуюся к выбранному личному диалогу.
    pub(crate) fn error_for_conversation(&self, conversation_id: &str) -> Option<String> {
        match self.state() {
            DirectCallUiState::Error {
                target,
                call,
                message,
            } if target
                .as_ref()
                .is_some_and(|target| target.room_id == conversation_id)
                || call
                    .as_ref()
                    .is_some_and(|call| call.conversation_id == conversation_id) =>
            {
                Some(message)
            }
            _ => None,
        }
    }

    /// Возвращает входящий звонок, ожидающий ответа текущего пользователя.
    pub(crate) fn incoming_call(&self) -> Option<DirectCallSnapshot> {
        let call = match self.state() {
            DirectCallUiState::Call(call) => call,
            DirectCallUiState::Error {
                call: Some(call), ..
            } => call,
            _ => return None,
        };
        (call.state == DirectCallState::Ringing && call.callee_user_id == self.current_user_id)
            .then_some(call)
    }

    /// Возвращает имя собеседника для указанного звонка.
    pub(crate) fn peer_nickname(&self, call: &DirectCallSnapshot) -> String {
        if call.caller_user_id == self.current_user_id {
            call.callee_nickname.clone()
        } else {
            call.caller_nickname.clone()
        }
    }

    /// Сообщает, является ли звонок исходящим для текущего пользователя.
    pub(crate) fn is_outgoing(&self, call: &DirectCallSnapshot) -> bool {
        call.caller_user_id == self.current_user_id
    }

    /// Начинает личный звонок.
    pub(crate) fn start(&self, target: VoiceRoomTarget) {
        if self.busy() {
            return;
        }
        if !matches!(
            self.state(),
            DirectCallUiState::Idle | DirectCallUiState::Error { call: None, .. }
        ) {
            let mut state = self.state;
            state.set(DirectCallUiState::Error {
                target: Some(target),
                call: self.current_call(),
                message: "Сначала заверши текущий личный звонок.".to_owned(),
            });
            return;
        }

        let realtime = self.realtime.clone();
        let handle = self.clone();
        let conversation_id = target.room_id.clone();
        let mut state = self.state;
        let mut busy = self.busy;
        state.set(DirectCallUiState::Starting {
            target: target.clone(),
        });
        busy.set(true);
        info!(
            %conversation_id,
            "starting direct call"
        );
        spawn(async move {
            match direct_call_realtime::start(&realtime, conversation_id.clone()).await {
                Ok(call) => handle.apply_snapshot(call),
                Err(error) => {
                    warn!(%error, %conversation_id, "failed to start direct call");
                    state.set(DirectCallUiState::Error {
                        target: Some(target),
                        call: None,
                        message: "Не удалось начать звонок. Проверь соединение и попробуй ещё раз."
                            .to_owned(),
                    });
                }
            }
            busy.set(false);
        });
    }

    /// Принимает входящий личный звонок.
    pub(crate) fn accept(&self) {
        self.respond(DirectCallResponse::Accept);
    }

    /// Отклоняет входящий личный звонок.
    pub(crate) fn decline(&self) {
        self.respond(DirectCallResponse::Decline);
    }

    /// Отменяет исходящий личный звонок до ответа.
    pub(crate) fn cancel(&self) {
        let Some(call) = self.current_call() else {
            return;
        };
        if call.state != DirectCallState::Ringing || !self.is_outgoing(&call) || self.busy() {
            return;
        }

        let realtime = self.realtime.clone();
        let handle = self.clone();
        let call_id = call.call_id.clone();
        let mut busy = self.busy;
        busy.set(true);
        info!(%call_id, "cancelling outgoing direct call");
        spawn(async move {
            match direct_call_realtime::cancel(&realtime, call_id.clone()).await {
                Ok(call) => handle.apply_snapshot(call),
                Err(error) => handle.apply_action_error(call, error.to_string(), "cancel"),
            }
            busy.set(false);
        });
    }

    /// Завершает принятый личный звонок.
    pub(crate) fn end(&self) {
        let Some(call) = self.current_call() else {
            return;
        };
        if call.state != DirectCallState::Active || self.busy() {
            return;
        }

        let realtime = self.realtime.clone();
        let handle = self.clone();
        let call_id = call.call_id.clone();
        let mut busy = self.busy;
        busy.set(true);
        info!(%call_id, "ending active direct call");
        spawn(async move {
            match direct_call_realtime::end(&realtime, call_id.clone()).await {
                Ok(call) => handle.apply_snapshot(call),
                Err(error) => handle.apply_action_error(call, error.to_string(), "end"),
            }
            busy.set(false);
        });
    }

    /// Завершает звонок, если цель медиа относится к нему.
    pub(crate) fn end_for_target(&self, target: &VoiceRoomTarget) -> bool {
        if target.kind != VoiceRoomTargetKind::DirectMessage {
            return false;
        }
        let Some(call) = self.current_call() else {
            return false;
        };
        if call.conversation_id != target.room_id || call.state != DirectCallState::Active {
            return false;
        }
        self.end();
        true
    }

    /// Применяет адресное lifecycle-событие.
    pub(crate) fn apply_event(&self, event: DirectCallLifecycleEvent) {
        if event.recipient_user_id != self.current_user_id {
            return;
        }
        self.apply_snapshot(event.call);
    }

    /// Восстанавливает незавершённый звонок после подключения realtime.
    pub(crate) fn recover(&self) {
        let realtime = self.realtime.clone();
        let handle = self.clone();
        spawn(async move {
            match direct_call_realtime::list(&realtime).await {
                Ok(snapshot) => {
                    let mut calls = snapshot.calls;
                    let call = calls
                        .iter()
                        .position(|call| call.state == DirectCallState::Active)
                        .map(|index| calls.remove(index))
                        .or_else(|| calls.into_iter().next());
                    if let Some(call) = call {
                        handle.apply_snapshot(call);
                    } else if !matches!(handle.state(), DirectCallUiState::Starting { .. }) {
                        let mut state = handle.state;
                        state.set(DirectCallUiState::Idle);
                    }
                }
                Err(error) => {
                    warn!(%error, "failed to recover direct calls");
                }
            }
        });
    }

    fn respond(&self, response: DirectCallResponse) {
        let Some(call) = self.incoming_call() else {
            return;
        };
        if self.busy() {
            return;
        }

        let realtime = self.realtime.clone();
        let handle = self.clone();
        let call_id = call.call_id.clone();
        let mut busy = self.busy;
        busy.set(true);
        info!(%call_id, ?response, "responding to incoming direct call");
        spawn(async move {
            match direct_call_realtime::respond(&realtime, call_id.clone(), response).await {
                Ok(call) => handle.apply_snapshot(call),
                Err(error) => handle.apply_action_error(call, error.to_string(), "respond"),
            }
            busy.set(false);
        });
    }

    fn apply_snapshot(&self, call: DirectCallSnapshot) {
        info!(
            call_id = %call.call_id,
            conversation_id = %call.conversation_id,
            state = ?call.state,
            end_reason = ?call.end_reason,
            "applying direct call lifecycle state"
        );
        let target = self.target_for(&call);
        let mut state = self.state;
        state.set(DirectCallUiState::Call(call.clone()));

        match call.state {
            DirectCallState::Active => {
                if !self
                    .voice
                    .state()
                    .active_target()
                    .is_some_and(|active| active.matches(&target))
                {
                    self.voice.join(target);
                }
            }
            DirectCallState::Ended => {
                if self
                    .voice
                    .state()
                    .active_target()
                    .is_some_and(|active| active.matches(&target))
                {
                    self.voice.leave();
                }
                let mut ended_state = self.state;
                let call_id = call.call_id;
                spawn(async move {
                    crate::features::runtime::sleep_ms(ENDED_STATE_VISIBLE_MS).await;
                    if matches!(
                        ended_state(),
                        DirectCallUiState::Call(current)
                            if current.call_id == call_id && current.state == DirectCallState::Ended
                    ) {
                        ended_state.set(DirectCallUiState::Idle);
                    }
                });
            }
            DirectCallState::Ringing => {}
        }
    }

    fn apply_action_error(&self, call: DirectCallSnapshot, error: String, action: &'static str) {
        warn!(
            %error,
            call_id = %call.call_id,
            conversation_id = %call.conversation_id,
            %action,
            "direct call action failed"
        );
        let mut state = self.state;
        state.set(DirectCallUiState::Error {
            target: None,
            call: Some(call),
            message: "Не удалось выполнить действие со звонком. Проверь соединение и повтори."
                .to_owned(),
        });
    }

    fn current_call(&self) -> Option<DirectCallSnapshot> {
        match self.state() {
            DirectCallUiState::Call(call) => Some(call),
            DirectCallUiState::Error {
                call: Some(call), ..
            } => Some(call),
            _ => None,
        }
    }

    fn target_for(&self, call: &DirectCallSnapshot) -> VoiceRoomTarget {
        VoiceRoomTarget::direct_message(call.conversation_id.clone(), self.peer_nickname(call))
    }
}
