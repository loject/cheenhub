//! User avatar initial helpers.

/// Returns the display initial for a user nickname.
pub(crate) fn user_initial(nickname: &str) -> String {
    nickname.chars().next().map_or_else(
        || "?".to_owned(),
        |character| character.to_uppercase().collect(),
    )
}
