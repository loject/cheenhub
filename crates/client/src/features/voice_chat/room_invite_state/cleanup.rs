//! Удаление завершённых приглашений после срока локального хранения.

use std::time::Duration;

use cheenhub_contracts::realtime::{VoiceRoomInviteSnapshot, VoiceRoomInviteStatus};
use dioxus::prelude::*;
use web_time::Instant;

use super::RoomInviteHandle;

/// Согласовано с двухминутным сроком хранения завершённых приглашений на сервере.
const TERMINAL_RETENTION: Duration = Duration::from_secs(120);

/// Удаляет только завершённое приглашение указанного ID.
fn remove_terminal_invite(invites: &mut Vec<VoiceRoomInviteSnapshot>, invite_id: &str) -> bool {
    let before = invites.len();
    invites.retain(|invite| {
        invite.invite_id != invite_id || invite.status == VoiceRoomInviteStatus::Pending
    });
    invites.len() != before
}

impl RoomInviteHandle {
    /// Планирует единственную очистку для каждого завершённого приглашения.
    pub(super) fn schedule_terminal_cleanup(&self, invite_id: &str) {
        if self
            .terminal_cleanup_deadlines
            .peek()
            .contains_key(invite_id)
        {
            return;
        }
        let deadline = Instant::now() + TERMINAL_RETENTION;
        let invite_id = invite_id.to_owned();
        let mut deadlines = self.terminal_cleanup_deadlines;
        deadlines.write().insert(invite_id.clone(), deadline);
        let handle = self.clone();
        spawn(async move {
            crate::features::runtime::sleep_ms(120_000).await;
            if handle.terminal_cleanup_deadlines.peek().get(&invite_id) != Some(&deadline) {
                return;
            }
            let mut deadlines = handle.terminal_cleanup_deadlines;
            deadlines.write().remove(&invite_id);
            let mut invites = handle.invites;
            let mut current = invites.peek().clone();
            if remove_terminal_invite(&mut current, &invite_id) {
                invites.set(current);
                debug!(%invite_id, "removed retained terminal voice room invitation");
            }
        });
    }
}

#[cfg(test)]
mod tests;
