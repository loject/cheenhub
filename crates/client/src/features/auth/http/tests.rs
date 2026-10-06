use reqwest::header::{HeaderMap, USER_AGENT};

use super::{attach_platform_user_agent, native};

#[test]
fn attaches_platform_user_agent_only_when_available() {
    let mut headers = HeaderMap::new();

    attach_platform_user_agent(&mut headers);

    let actual = headers
        .get(USER_AGENT)
        .and_then(|value| value.to_str().ok());

    assert_eq!(actual, native::client_user_agent());
}
