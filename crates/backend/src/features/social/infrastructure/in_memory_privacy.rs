//! Локальное хранение приватности и истории блокировок.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::InMemorySocialStore;
use super::in_memory::poisoned;
use super::privacy_store::PrivacyStore;
use crate::features::social::domain::{FriendshipStatus, PrivacySettings, UserBlock, ordered_pair};

#[async_trait]
impl PrivacyStore for InMemorySocialStore {
    async fn privacy_settings(&self, user_id: &Uuid) -> anyhow::Result<PrivacySettings> {
        Ok(self
            .privacy_settings
            .lock()
            .map_err(|_| poisoned())?
            .get(user_id)
            .copied()
            .unwrap_or_default())
    }

    async fn save_privacy_settings(
        &self,
        user_id: &Uuid,
        settings: PrivacySettings,
        _now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        self.privacy_settings
            .lock()
            .map_err(|_| poisoned())?
            .insert(*user_id, settings);
        Ok(())
    }

    async fn user_blocks_between(
        &self,
        left: &Uuid,
        right: &Uuid,
    ) -> anyhow::Result<Vec<UserBlock>> {
        Ok(self
            .user_blocks
            .lock()
            .map_err(|_| poisoned())?
            .iter()
            .filter(|block| {
                (block.blocker_user_id == *left && block.blocked_user_id == *right)
                    || (block.blocker_user_id == *right && block.blocked_user_id == *left)
            })
            .cloned()
            .collect())
    }

    async fn active_user_blocks(&self, user_id: &Uuid) -> anyhow::Result<Vec<UserBlock>> {
        let mut blocks = self
            .user_blocks
            .lock()
            .map_err(|_| poisoned())?
            .iter()
            .filter(|block| block.blocker_user_id == *user_id && block.unblocked_at.is_none())
            .cloned()
            .collect::<Vec<_>>();
        blocks.sort_by_key(|block| std::cmp::Reverse(block.blocked_at));
        Ok(blocks)
    }

    async fn block_user(
        &self,
        blocker: &Uuid,
        blocked: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<UserBlock> {
        let mut blocks = self.user_blocks.lock().map_err(|_| poisoned())?;
        let block = if let Some(existing) = blocks
            .iter_mut()
            .find(|block| block.blocker_user_id == *blocker && block.blocked_user_id == *blocked)
        {
            if existing.unblocked_at.is_some() {
                existing.blocked_at = now;
                existing.unblocked_at = None;
            }
            existing.clone()
        } else {
            let block = UserBlock {
                id: Uuid::new_v4(),
                blocker_user_id: *blocker,
                blocked_user_id: *blocked,
                blocked_at: now,
                unblocked_at: None,
            };
            blocks.push(block.clone());
            block
        };
        let (low, high) = ordered_pair(*blocker, *blocked);
        let mut friendships = self.friendships.lock().map_err(|_| poisoned())?;
        for friendship in friendships
            .iter_mut()
            .filter(|friendship| friendship.user_low_id == low && friendship.user_high_id == high)
        {
            if matches!(
                friendship.status,
                FriendshipStatus::Pending | FriendshipStatus::Accepted
            ) {
                friendship.status = FriendshipStatus::Cancelled;
                friendship.updated_at = now;
            }
        }
        Ok(block)
    }

    async fn unblock_user(
        &self,
        blocker: &Uuid,
        blocked: &Uuid,
        now: DateTime<Utc>,
    ) -> anyhow::Result<bool> {
        let mut blocks = self.user_blocks.lock().map_err(|_| poisoned())?;
        let Some(block) = blocks.iter_mut().find(|block| {
            block.blocker_user_id == *blocker
                && block.blocked_user_id == *blocked
                && block.unblocked_at.is_none()
        }) else {
            return Ok(false);
        };
        block.unblocked_at = Some(now);
        Ok(true)
    }

    async fn has_dm_message_after(
        &self,
        conversation_id: &Uuid,
        author_id: &Uuid,
        after: DateTime<Utc>,
    ) -> anyhow::Result<bool> {
        Ok(self
            .messages
            .lock()
            .map_err(|_| poisoned())?
            .iter()
            .any(|message| {
                message.conversation_id == *conversation_id
                    && message.sender_user_id == *author_id
                    && message.created_at > after
            }))
    }
}
