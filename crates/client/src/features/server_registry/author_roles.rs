//! Ограниченный клиентский кэш ролей авторов сообщений.

use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Duration;
use web_time::Instant;

pub(super) const CACHE_LIMIT: usize = 2048;
const POSITIVE_TTL: Duration = Duration::from_secs(300);
const NEGATIVE_TTL: Duration = Duration::from_secs(30);

type AuthorKey = (String, String);

struct CachedRoles {
    ids: Vec<String>,
    expires_at: Instant,
}

#[derive(Default)]
pub(super) struct AuthorRoleCache {
    entries: HashMap<AuthorKey, CachedRoles>,
    order: VecDeque<AuthorKey>,
    generations: HashMap<String, u64>,
}

impl AuthorRoleCache {
    pub(super) fn generation(&self, server_id: &str) -> u64 {
        self.generations.get(server_id).copied().unwrap_or(0)
    }

    /// Пустой срез означает проверенного автора без активных ролей.
    pub(super) fn roles(&self, server_id: &str, author_id: &str) -> Option<&[String]> {
        let entry = self
            .entries
            .get(&(server_id.to_owned(), author_id.to_owned()))?;
        (entry.expires_at > Instant::now()).then_some(entry.ids.as_slice())
    }

    /// Выделяет неизвестных авторов, не повторяя идентификаторы в запросе.
    pub(super) fn unresolved(&self, server_id: &str, authors: &[String]) -> Vec<String> {
        let mut seen = HashSet::new();
        authors
            .iter()
            .filter(|id| seen.insert(id.as_str()) && self.roles(server_id, id).is_none())
            .cloned()
            .collect()
    }

    /// Снимок применяется только при неизменном поколении ролей сервера.
    pub(super) fn absorb_page(
        &mut self,
        server_id: &str,
        generation: u64,
        author_ids: &[String],
        assignments: &HashMap<String, Vec<String>>,
    ) -> bool {
        if self.generation(server_id) != generation {
            return false;
        }
        for author_id in author_ids {
            self.set_user(
                server_id,
                author_id,
                assignments.get(author_id).cloned().unwrap_or_default(),
            );
        }
        true
    }

    /// Устанавливает как положительные, так и отрицательные результаты разрешения.
    pub(super) fn set_user(&mut self, server_id: &str, author_id: &str, ids: Vec<String>) {
        let key = (server_id.to_owned(), author_id.to_owned());
        let ttl = if ids.is_empty() {
            NEGATIVE_TTL
        } else {
            POSITIVE_TTL
        };
        if !self.entries.contains_key(&key) {
            self.order.push_back(key.clone());
        }
        self.entries.insert(
            key,
            CachedRoles {
                ids,
                expires_at: Instant::now() + ttl,
            },
        );
        while self.entries.len() > CACHE_LIMIT {
            if let Some(oldest) = self.order.pop_front() {
                self.entries.remove(&oldest);
            } else {
                break;
            }
        }
    }

    pub(super) fn invalidate_user(&mut self, server_id: &str, author_id: &str) {
        self.advance(server_id);
        let key = (server_id.to_owned(), author_id.to_owned());
        self.entries.remove(&key);
        self.order.retain(|saved| saved != &key);
    }

    pub(super) fn invalidate_server(&mut self, server_id: &str) {
        self.advance(server_id);
        self.entries.retain(|(server, _), _| server != server_id);
        self.order.retain(|(server, _)| server != server_id);
    }

    fn advance(&mut self, server_id: &str) {
        let generation = self.generations.entry(server_id.to_owned()).or_default();
        *generation = generation.wrapping_add(1);
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests;
