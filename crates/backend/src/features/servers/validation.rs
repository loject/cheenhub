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
}
