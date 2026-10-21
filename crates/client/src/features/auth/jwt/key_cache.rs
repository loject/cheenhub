//! Кеш публичного JWT-ключа и объединение параллельных загрузок внутри auth.

use std::{cell::RefCell, future::Future, rc::Rc};

use cheenhub_contracts::rest::AuthVerificationKey;
use futures_util::{
    FutureExt,
    future::{LocalBoxFuture, Shared},
};

use super::JwtVerifyError;

type KeyResult = Result<AuthVerificationKey, JwtVerifyError>;
type KeyRequest = Shared<LocalBoxFuture<'static, KeyResult>>;

thread_local! {
    static CACHE: Rc<VerificationKeyCache> = Rc::default();
}

#[derive(Default)]
struct State {
    key: Option<AuthVerificationKey>,
    pending: Option<KeyRequest>,
}

#[derive(Default)]
struct VerificationKeyCache {
    state: RefCell<State>,
}

impl VerificationKeyCache {
    async fn load(&self, fetch: impl Future<Output = KeyResult> + 'static) -> KeyResult {
        let request = {
            let mut state = self.state.borrow_mut();
            if let Some(key) = &state.key {
                return Ok(key.clone());
            }
            state
                .pending
                .get_or_insert_with(|| fetch.boxed_local().shared())
                .clone()
        };
        let result = request.clone().await;
        let mut state = self.state.borrow_mut();
        // Поздний потребитель старого ответа не должен сбрасывать новую попытку загрузки.
        if state
            .pending
            .as_ref()
            .is_some_and(|pending| pending.ptr_eq(&request))
        {
            state.pending = None;
            if let Ok(key) = &result {
                state.key = Some(key.clone());
            }
        }
        result
    }
}

/// Возвращает ключ из кеша auth или ожидает общую загрузку для текущего клиента.
///
/// Только успешная проверенная загрузка сохраняется до закрытия вкладки/окна.
/// Ошибки получают все ожидающие потребители, последующий вызов повторяет запрос.
pub(super) async fn load() -> KeyResult {
    let cache = CACHE.with(Rc::clone);
    cache.load(super::fetch_verification_key()).await
}

#[cfg(test)]
mod tests;
