//! Предпочтения PulseAudio отделены от несовместимых сохранённых ALSA PCM имён.

/// Ключ системного имени PulseAudio source или sink.
pub(crate) const DEVICE_ID_KEY: &str = "cheenhub.microphone.pulse.input_device_id";
/// Ключ пользовательского описания устройства.
pub(crate) const DEVICE_LABEL_KEY: &str = "cheenhub.microphone.pulse.input_device_label";
/// Одинаковые описания не означают одинаковые устройства.
pub(crate) const RECOVER_BY_LABEL: bool = false;
