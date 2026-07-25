//! Заглушка системного входа через Google для неподдерживаемых платформ.

use crate::features::auth::google_sign_in::GoogleSignInError;

pub(in crate::features::auth::google_sign_in) const fn is_supported() -> bool {
    false
}

pub(in crate::features::auth::google_sign_in) async fn request_google_id_token(
    _server_client_id: String,
    _nonce: String,
) -> Result<Option<String>, GoogleSignInError> {
    Err(GoogleSignInError::new(
        "Системный вход через Google недоступен на этой платформе",
    ))
}
