//! Realtime-подключение к журналу бэкенда.

mod backend;
mod platform;

pub(super) use backend::run;
