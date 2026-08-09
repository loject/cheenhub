//! Контракт платформенного перечисления устройств вывода аудио.

mod contract;
mod native;

pub(crate) use contract::{AudioOutputDevice, AudioOutputDevicesResult};
pub(crate) use native::enumerate_audio_output_devices;
