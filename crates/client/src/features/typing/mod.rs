//! Индикатор набора сообщения в комнатах и личных диалогах.
//!
//! Фича не владеет формой сообщения: она получает от composer сигнал о вводе и
//! отдает наружу `Callback`, который вызывающий чат подменяет на отправку
//! realtime-события для своего модуля. Так комнаты и личные диалоги используют
//! общий троттлинг и жизненный цикл, не дублируя их в каждом чате.

mod indicator;
mod realtime;
mod state;

pub(crate) use indicator::TypingIndicator;
pub(crate) use realtime::{TyperEntry, TypingIntent, TypingTarget, TypingTransport};
pub(crate) use state::{TypingNotifier, use_typing_notifier, use_typing_participants};
