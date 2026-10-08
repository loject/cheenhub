//! Выбор формата native-захвата без зависимости от порядка перечисления CPAL.

use cpal::{SampleFormat, SampleRate, SupportedStreamConfig, SupportedStreamConfigRange};

/// Выбирает наиболее точный поддерживаемый PCM на заданной частоте.
///
/// При одинаковой точности предпочитает требуемое число каналов; остальные
/// каналы преобразуются в mono существующим capture callback.
pub(super) fn select(
    configs: impl IntoIterator<Item = SupportedStreamConfigRange>,
    target_rate: SampleRate,
    channels: u16,
) -> Option<SupportedStreamConfig> {
    configs
        .into_iter()
        .filter_map(|range| {
            if range.min_sample_rate() > target_rate || range.max_sample_rate() < target_rate {
                return None;
            }
            let quality = format_quality(range.sample_format())?;
            let candidate = range.with_sample_rate(target_rate);
            let matches_channels = candidate.channels() == channels;
            Some(((quality, matches_channels), candidate))
        })
        .max_by_key(|(rank, _)| *rank)
        .map(|(_, config)| config)
}

fn format_quality(format: SampleFormat) -> Option<u8> {
    // I64/U64 не поддерживаются build_input_stream; не выбираем их для захвата.
    match format {
        SampleFormat::F32 => Some(8),
        SampleFormat::F64 => Some(7),
        SampleFormat::I32 => Some(6),
        SampleFormat::U32 => Some(5),
        SampleFormat::I16 => Some(4),
        SampleFormat::U16 => Some(3),
        SampleFormat::I8 => Some(2),
        SampleFormat::U8 => Some(1),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
