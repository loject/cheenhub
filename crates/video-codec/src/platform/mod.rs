//! Платформенная граница encoder: аппаратные варианты и программный резерв.

mod native;
pub(super) use native::{Candidate, candidates, configure, upload};
