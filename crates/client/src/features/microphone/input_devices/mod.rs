//! Контракт платформенного перечисления устройств ввода аудио.

mod native;
mod web;

pub(crate) use native::{
    AudioInputDevice, AudioInputDevicesResult, enumerate_audio_input_devices,
    request_microphone_permission,
};
