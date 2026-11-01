//! Отдельные плитки исходящих приглашений, ещё не являющихся участниками комнаты.

use std::collections::{HashMap, HashSet};

use cheenhub_contracts::realtime::{
    VoiceRoomInviteCandidate, VoiceRoomInviteSnapshot, VoiceRoomInviteStatus,
};
use web_time::Instant;

/// Визуальное состояние приглашённого пользователя в сетке комнаты.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum OutgoingInviteTileStatus {
    Calling,
    AcceptedWaiting,
    Unanswered,
    Declined,
    Cancelled,
}

/// Данные плитки, не подменяющей реального участника голосовой комнаты.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct OutgoingInviteTile {
    pub(super) invite_id: String,
    pub(super) recipient_user_id: String,
    pub(super) nickname: String,
    pub(super) avatar_url: Option<String>,
    pub(super) status: OutgoingInviteTileStatus,
}

/// Кратковременный результат завершившегося приглашения.
#[derive(Clone, PartialEq, Eq)]
pub(super) struct RecentOutgoingResult {
    pub(super) invite: VoiceRoomInviteSnapshot,
    pub(super) show_until: Instant,
}

/// Фильтр исходящих приглашений по отправителю и выбранной комнате.
#[derive(Clone, Copy)]
pub(super) struct OutgoingInviteScope<'a> {
    pub(super) caller_id: &'a str,
    pub(super) server_id: &'a str,
    pub(super) room_id: &'a str,
}

/// Выбирает ожидающие приглашения и результаты, ещё видимые в комнате.
pub(super) fn outgoing_invite_tiles(
    invites: &[VoiceRoomInviteSnapshot],
    pending_deadlines: &HashMap<String, Instant>,
    results: &HashMap<String, RecentOutgoingResult>,
    candidates: &HashMap<String, VoiceRoomInviteCandidate>,
    scope: OutgoingInviteScope<'_>,
    present_users: &HashSet<String>,
    now: Instant,
) -> Vec<OutgoingInviteTile> {
    let mut by_recipient = HashMap::<String, (String, OutgoingInviteTile)>::new();
    let mut collect = |invite: &VoiceRoomInviteSnapshot, status| {
        if invite.caller_user_id != scope.caller_id
            || invite.server_id != scope.server_id
            || invite.room_id != scope.room_id
            || present_users.contains(&invite.recipient_user_id)
        {
            return;
        }
        let replace =
            by_recipient
                .get(&invite.recipient_user_id)
                .is_none_or(|(created_at, existing)| {
                    created_at < &invite.created_at
                        || (created_at == &invite.created_at
                            && status == OutgoingInviteTileStatus::Calling
                            && existing.status != OutgoingInviteTileStatus::Calling)
                });
        if !replace {
            return;
        }
        let candidate = candidates.get(&invite.recipient_user_id);
        let nickname = candidate
            .map(|candidate| candidate.nickname.clone())
            .or_else(|| invite.recipient_nickname.clone())
            .unwrap_or_else(|| "Приглашённый участник".to_owned());
        let avatar_url = candidate
            .and_then(|candidate| candidate.avatar_url.clone())
            .or_else(|| invite.recipient_avatar_url.clone());
        by_recipient.insert(
            invite.recipient_user_id.clone(),
            (
                invite.created_at.clone(),
                OutgoingInviteTile {
                    invite_id: invite.invite_id.clone(),
                    recipient_user_id: invite.recipient_user_id.clone(),
                    nickname,
                    avatar_url,
                    status,
                },
            ),
        );
    };
    for invite in invites {
        if invite.status == VoiceRoomInviteStatus::Pending
            && pending_deadlines
                .get(&invite.invite_id)
                .is_some_and(|deadline| *deadline > now)
        {
            collect(invite, OutgoingInviteTileStatus::Calling);
        }
    }
    for result in results.values() {
        if result.show_until <= now {
            continue;
        }
        let status = match result.invite.status {
            VoiceRoomInviteStatus::Accepted => OutgoingInviteTileStatus::AcceptedWaiting,
            VoiceRoomInviteStatus::Expired => OutgoingInviteTileStatus::Unanswered,
            VoiceRoomInviteStatus::Declined => OutgoingInviteTileStatus::Declined,
            VoiceRoomInviteStatus::Cancelled => OutgoingInviteTileStatus::Cancelled,
            _ => continue,
        };
        collect(&result.invite, status);
    }
    let mut ordered = by_recipient.into_values().collect::<Vec<_>>();
    ordered.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then(left.1.recipient_user_id.cmp(&right.1.recipient_user_id))
    });
    ordered.into_iter().map(|(_, tile)| tile).collect()
}

#[cfg(test)]
mod tests;
