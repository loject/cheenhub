//! Контракт платформенного перечисления устройств вывода аудио.

mod native;
mod web;

pub(crate) use native::{
    AudioOutputDevice, AudioOutputDevicesResult, enumerate_audio_output_devices,
};
