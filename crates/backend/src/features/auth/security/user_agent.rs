//! User-Agent normalization helpers.

const MAX_USER_AGENT_CHARS: usize = 512;

/// Returns a bounded User-Agent value suitable for persistence.
pub(crate) fn normalize(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.chars().take(MAX_USER_AGENT_CHARS).collect())
}
