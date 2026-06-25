//! Вспомогательные функции хеширования паролей.

use std::sync::OnceLock;

use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};

/// Возвращает настроенный экземпляр Argon2id с явными параметрами стоимости.
///
/// Параметры зафиксированы согласно текущим рекомендациям OWASP
/// (m = 19 MiB, t = 2, p = 1), чтобы стойкость не зависела молча от дефолтов
/// версии крейта при будущих обновлениях зависимостей. Проверка уже сохраненных
/// хешей остается совместимой, так как verify использует параметры из самой
/// PHC-строки.
fn argon2() -> Argon2<'static> {
    static PARAMS: OnceLock<Params> = OnceLock::new();
    let params = PARAMS
        .get_or_init(|| Params::new(19 * 1024, 2, 1, None).expect("valid argon2 params"))
        .clone();
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

/// Хеширует обычный пароль с помощью Argon2id.
pub(crate) fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Ok(argon2()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|error| anyhow::anyhow!("failed to hash password: {error}"))?
        .to_string())
}

/// Проверяет обычный пароль по сохраненному хешу Argon2.
pub(crate) fn verify_password(password: &str, hash: &str) -> bool {
    let Ok(parsed_hash) = PasswordHash::new(hash) else {
        return false;
    };

    argon2()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

/// Выполняет проверку пароля против фиктивного хеша.
///
/// Вызывается на ветках логина, где пользователь не найден или у него нет
/// локального пароля. Без этого такие ветки возвращались бы заметно быстрее
/// (без работы Argon2), позволяя перечислять существующие аккаунты по таймингу.
pub(crate) fn verify_dummy_password() {
    static DUMMY_HASH: OnceLock<String> = OnceLock::new();
    let dummy =
        DUMMY_HASH.get_or_init(|| hash_password("cheenhub-timing-equalizer").unwrap_or_default());
    let _ = verify_password("cheenhub-timing-equalizer-probe", dummy);
}
