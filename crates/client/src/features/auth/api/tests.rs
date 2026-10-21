use serde_json::json;

use super::{OAuthCompletion, parse_oauth_completion};

#[test]
fn parses_native_google_registration_handoff() {
    let key = cheenhub_contracts::rest::AuthVerificationKey {
        key_id: "unused".to_owned(),
        public_key_base64: String::new(),
    };
    let completion = parse_oauth_completion(
        json!({
            "kind": "registration_required",
            "registration_token": "native-registration-token",
            "email": "person@example.com",
            "display_name": "Person"
        }),
        &key,
    )
    .expect("ответ нативной регистрации должен разбираться");

    let OAuthCompletion::RegistrationRequired(registration) = completion else {
        panic!("ожидался запрос завершения регистрации");
    };
    assert_eq!(registration.registration_token, "native-registration-token");
    assert_eq!(registration.email.as_deref(), Some("person@example.com"));
    assert_eq!(registration.suggested_nickname.as_deref(), Some("Person"));
}
