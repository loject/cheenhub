//! Server input validation.

/// Normalized create-server input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ValidCreateServer {
    /// Human-readable server name.
    pub(crate) name: String,
}

/// Validates and normalizes server creation input.
pub(crate) fn create_server(name: String) -> Result<ValidCreateServer, &'static str> {
    let name = name.trim().to_owned();
    let len = name.chars().count();

    if !(2..=48).contains(&len) {
        return Err("Название сервера должно быть длиной от 2 до 48 символов.");
    }

    Ok(ValidCreateServer { name })
}

/// Normalized create-invite input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ValidCreateServerInvite {
    /// Optional maximum number of accepted invite uses.
    pub(crate) max_uses: Option<u32>,
    /// Optional invite lifetime in days.
    pub(crate) expires_in_days: Option<u32>,
}

/// Validates server invite creation input.
pub(crate) fn create_server_invite(
    max_uses: Option<u32>,
    expires_in_days: Option<u32>,
) -> Result<ValidCreateServerInvite, &'static str> {
    if matches!(max_uses, Some(0 | 1000..)) {
        return Err("Лимит использований должен быть от 1 до 999.");
    }

    if matches!(expires_in_days, Some(0 | 366..)) {
        return Err("Срок действия должен быть от 1 до 365 дней.");
    }

    Ok(ValidCreateServerInvite {
        max_uses,
        expires_in_days,
    })
}

#[cfg(test)]
mod tests {
    use super::create_server;

    #[test]
    fn trims_valid_server_name() {
        let valid = create_server("  CheenHub Dev  ".to_owned()).expect("name should be valid");

        assert_eq!(valid.name, "CheenHub Dev");
    }

    #[test]
    fn rejects_empty_server_name() {
        assert!(create_server("   ".to_owned()).is_err());
    }

    #[test]
    fn rejects_short_server_name() {
        assert!(create_server("a".to_owned()).is_err());
    }

    #[test]
    fn rejects_long_server_name() {
        assert!(create_server("a".repeat(49)).is_err());
    }

    #[test]
    fn accepts_valid_invite_settings() {
        let valid = super::create_server_invite(Some(30), Some(7))
            .expect("invite settings should be valid");

        assert_eq!(valid.max_uses, Some(30));
        assert_eq!(valid.expires_in_days, Some(7));
    }

    #[test]
    fn rejects_invalid_invite_usage_limit() {
        assert!(super::create_server_invite(Some(0), None).is_err());
        assert!(super::create_server_invite(Some(1000), None).is_err());
    }

    #[test]
    fn rejects_invalid_invite_expiration() {
        assert!(super::create_server_invite(None, Some(0)).is_err());
        assert!(super::create_server_invite(None, Some(366)).is_err());
    }
}
