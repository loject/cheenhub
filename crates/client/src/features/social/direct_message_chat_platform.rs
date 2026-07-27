//! Платформенная политика начальной видимости чата во время личного звонка.

mod native;

pub(super) use native::open_by_default;
