//! Контракт платформенного перечисления устройств ввода аудио.

mod contract;
mod native;

pub(crate) use contract::{AudioInputDevice, AudioInputDevicesResult};
pub(crate) use native::{enumerate_audio_input_devices, request_microphone_permission};
