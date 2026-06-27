//! Преобразование PCM микшера в native audio samples.

pub(super) trait CpalOutputSample: Copy + Send + 'static {
    fn from_f32(sample: f32) -> Self;
}

impl CpalOutputSample for f32 {
    fn from_f32(sample: f32) -> Self {
        sample.clamp(-1.0, 1.0)
    }
}

impl CpalOutputSample for f64 {
    fn from_f32(sample: f32) -> Self {
        f64::from(sample.clamp(-1.0, 1.0))
    }
}

impl CpalOutputSample for i8 {
    fn from_f32(sample: f32) -> Self {
        (sample.clamp(-1.0, 1.0) * i8::MAX as f32) as Self
    }
}

impl CpalOutputSample for i16 {
    fn from_f32(sample: f32) -> Self {
        (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as Self
    }
}

impl CpalOutputSample for i32 {
    fn from_f32(sample: f32) -> Self {
        (sample.clamp(-1.0, 1.0) * i32::MAX as f32) as Self
    }
}

impl CpalOutputSample for u8 {
    fn from_f32(sample: f32) -> Self {
        (sample.clamp(-1.0, 1.0) * 128.0 + 128.0).clamp(0.0, u8::MAX as f32) as Self
    }
}

impl CpalOutputSample for u16 {
    fn from_f32(sample: f32) -> Self {
        (sample.clamp(-1.0, 1.0) * 32_768.0 + 32_768.0).clamp(0.0, u16::MAX as f32) as Self
    }
}

impl CpalOutputSample for u32 {
    fn from_f32(sample: f32) -> Self {
        (sample.clamp(-1.0, 1.0) * 2_147_483_648.0 + 2_147_483_648.0).clamp(0.0, u32::MAX as f32)
            as Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_f32_without_overflow() {
        assert_eq!(f32::from_f32(2.0), 1.0);
        assert_eq!(f32::from_f32(-2.0), -1.0);
    }

    #[test]
    fn converts_unsigned_midpoint_to_silence() {
        assert_eq!(u8::from_f32(0.0), 128);
        assert_eq!(u16::from_f32(0.0), 32_768);
    }
}
