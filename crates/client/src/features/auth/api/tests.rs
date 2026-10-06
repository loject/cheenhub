use serde_json::json;

use super::{OAuthCompletion, parse_oauth_completion};

#[test]
fn parses_native_google_registration_handoff() {
    let completion = parse_oauth_completion(json!({
        "kind": "registration_required",
        "registration_token": "native-registration-token",
        "email": "person@example.com",
        "display_name": "Person"
    }))
    .expect("ответ нативной регистрации должен разбираться");

    let OAuthCompletion::RegistrationRequired(registration) = completion else {
        panic!("ожидался запрос завершения регистрации");
    };
    assert_eq!(registration.registration_token, "native-registration-token");
    assert_eq!(registration.email.as_deref(), Some("person@example.com"));
    assert_eq!(registration.suggested_nickname.as_deref(), Some("Person"));
}
