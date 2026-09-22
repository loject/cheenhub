//! Подготовка кадров Windows Graphics Capture для VP9.

/// Плотно упакованный кадр I420 для программного VP9-кодировщика.
pub(super) struct I420Frame {
    pub(super) bytes: Vec<u8>,
}

impl I420Frame {
    /// Считывает BGRA-строки, сохраняет весь экран и заполняет поля чёрным.
    pub(super) fn from_bgra(
        source: &[u8],
        source_width: usize,
        source_height: usize,
        source_stride: usize,
        target_width: usize,
        target_height: usize,
    ) -> Result<Self, String> {
        if source_width == 0
            || source_height == 0
            || target_width == 0
            || target_height == 0
            || !target_width.is_multiple_of(2)
            || !target_height.is_multiple_of(2)
            || source_stride < source_width.saturating_mul(4)
            || source.len() < source_stride.saturating_mul(source_height)
        {
            return Err("некорректный размер кадра демонстрации экрана".to_owned());
        }

        let luma_len = target_width * target_height;
        let chroma_len = luma_len / 4;
        let mut bytes = vec![16; luma_len + chroma_len * 2];
        bytes[luma_len..].fill(128);
        let (image_width, image_height) =
            fit(source_width, source_height, target_width, target_height);
        let image_left = (target_width - image_width) / 2;
        let image_top = (target_height - image_height) / 2;

        for y in (0..target_height).step_by(2) {
            for x in (0..target_width).step_by(2) {
                let mut u_sum = 0_i32;
                let mut v_sum = 0_i32;
                for dy in 0..2 {
                    for dx in 0..2 {
                        let px = x + dx;
                        let py = y + dy;
                        let (red, green, blue) = if px >= image_left
                            && px < image_left + image_width
                            && py >= image_top
                            && py < image_top + image_height
                        {
                            sample_bilinear(
                                source,
                                source_width,
                                source_height,
                                source_stride,
                                (px - image_left) as f32 * source_width as f32 / image_width as f32,
                                (py - image_top) as f32 * source_height as f32
                                    / image_height as f32,
                            )
                        } else {
                            (0, 0, 0)
                        };
                        let red = i32::from(red);
                        let green = i32::from(green);
                        let blue = i32::from(blue);
                        bytes[py * target_width + px] =
                            (((66 * red + 129 * green + 25 * blue + 128) >> 8) + 16).clamp(0, 255)
                                as u8;
                        u_sum += (-38 * red - 74 * green + 112 * blue + 128) >> 8;
                        v_sum += (112 * red - 94 * green - 18 * blue + 128) >> 8;
                    }
                }
                let chroma_index = (y / 2) * (target_width / 2) + x / 2;
                bytes[luma_len + chroma_index] = (128 + u_sum / 4).clamp(0, 255) as u8;
                bytes[luma_len + chroma_len + chroma_index] = (128 + v_sum / 4).clamp(0, 255) as u8;
            }
        }
        Ok(Self { bytes })
    }
}

fn fit(source_width: usize, source_height: usize, width: usize, height: usize) -> (usize, usize) {
    if source_width as u64 * height as u64 > source_height as u64 * width as u64 {
        let fitted = source_height * width / source_width;
        (width, (fitted & !1).max(2).min(height))
    } else {
        let fitted = source_width * height / source_height;
        ((fitted & !1).max(2).min(width), height)
    }
}

fn sample_bilinear(
    source: &[u8],
    width: usize,
    height: usize,
    stride: usize,
    x: f32,
    y: f32,
) -> (u8, u8, u8) {
    let x0 = (x.floor() as usize).min(width - 1);
    let y0 = (y.floor() as usize).min(height - 1);
    let x1 = (x0 + 1).min(width - 1);
    let y1 = (y0 + 1).min(height - 1);
    let fx = x.fract();
    let fy = y.fract();
    let sample = |sx, sy, channel| f32::from(source[sy * stride + sx * 4 + channel]);
    let interpolate = |channel| {
        let top = sample(x0, y0, channel) * (1.0 - fx) + sample(x1, y0, channel) * fx;
        let bottom = sample(x0, y1, channel) * (1.0 - fx) + sample(x1, y1, channel) * fx;
        (top * (1.0 - fy) + bottom * fy).round() as u8
    };
    (interpolate(2), interpolate(1), interpolate(0))
}

#[cfg(test)]
mod tests {
    use super::I420Frame;

    #[test]
    fn converts_bgra_to_i420_at_selected_size() {
        let pixels = [
            0_u8, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255,
        ];
        let frame = I420Frame::from_bgra(&pixels, 2, 2, 8, 2, 2).unwrap();
        assert_eq!(frame.bytes.len(), 6);
        assert!(frame.bytes[..4].iter().all(|value| *value > 70));
    }

    #[test]
    fn preserves_full_source_with_black_bars() {
        let pixels = [255_u8; 64];
        let frame = I420Frame::from_bgra(&pixels, 4, 4, 16, 8, 4).unwrap();
        assert_eq!(frame.bytes.len(), 48);
        assert_eq!(&frame.bytes[..2], &[16, 16]);
        assert!(frame.bytes[2..6].iter().all(|value| *value > 200));
        assert_eq!(&frame.bytes[6..8], &[16, 16]);
    }

    #[test]
    fn rejects_short_source_rows() {
        assert!(I420Frame::from_bgra(&[0; 8], 2, 2, 8, 2, 2).is_err());
    }
}
