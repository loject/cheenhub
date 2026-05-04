//! Small domain values used by the authentication UI.

#[derive(Clone, Copy, PartialEq)]
pub(super) enum AuthProvider {
    Google,
    Discord,
}

impl AuthProvider {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Google => "Войти через Google",
            Self::Discord => "Войти через Discord",
        }
    }

    pub(super) fn badge(self) -> &'static str {
        match self {
            Self::Google => "G",
            Self::Discord => "D",
        }
    }
}
