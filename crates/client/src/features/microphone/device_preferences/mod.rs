//! Платформенные ключи и правила восстановления выбранного аудиоустройства.

mod native;

pub(super) use native::{DEVICE_ID_KEY, DEVICE_LABEL_KEY, RECOVER_BY_LABEL};
