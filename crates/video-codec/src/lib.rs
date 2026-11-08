//! Статический FFmpeg для аппаратного и программного VP9 и live WebM.
//!
//! Обёртки ffmpeg-next владеют контекстами. Захват, сеть и отображение остаются
//! в клиентских feature и не входят в контракт этой библиотеки.

#![warn(missing_docs)]

mod encoder;
mod muxer;
mod platform;

pub use encoder::{EncodedPacket, Vp9Encoder};
pub use muxer::WebmMuxer;

fn validate_dimensions(width: u32, height: u32) -> Result<(), String> {
    if width == 0
        || height == 0
        || width > 16_384
        || height > 16_384
        || !width.is_multiple_of(2)
        || !height.is_multiple_of(2)
    {
        return Err("Некорректное разрешение VP9".into());
    }
    Ok(())
}

fn check(result: i32) -> Result<(), String> {
    if result < 0 {
        return Err(ffmpeg_next::Error::from(result).to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
