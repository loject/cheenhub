//! Размеры изображения из несжатого заголовка ключевого VP9-кадра.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Размеры кодированного кадра и области его отображения в VP9.
pub(super) struct Dimensions {
    /// Ширина кодированного изображения в пикселях.
    pub(super) width: u32,
    /// Высота кодированного изображения в пикселях.
    pub(super) height: u32,
    /// Ширина изображения после применения render size.
    pub(super) display_width: u32,
    /// Высота изображения после применения render size.
    pub(super) display_height: u32,
}

/// Читает размеры ключевого VP9-кадра; отклоняет повреждённые заголовки.
pub(super) fn dimensions(bytes: &[u8]) -> Result<Dimensions, String> {
    let mut bits = Bits { bytes, offset: 0 };
    if bits.read(2)? != 2 {
        return Err("Некорректный frame marker VP9".into());
    }
    let profile = bits.read(1)? | (bits.read(1)? << 1);
    if profile == 3 && bits.read(1)? != 0 {
        return Err("Некорректный reserved bit VP9".into());
    }
    if bits.read(1)? != 0 || bits.read(1)? != 0 {
        return Err("Для запуска WebM нужен ключевой VP9-кадр".into());
    }
    bits.read(2)?;
    if bits.read(24)? != 0x498342 {
        return Err("Некорректный sync code VP9".into());
    }
    if profile >= 2 {
        bits.read(1)?;
    }
    let color_space = bits.read(3)?;
    if color_space != 7 {
        bits.read(1)?;
        if profile == 1 || profile == 3 {
            bits.read(3)?;
        }
    } else if profile == 1 || profile == 3 {
        bits.read(1)?;
    } else {
        return Err("Недопустимый RGB color space для профиля VP9".into());
    }
    let width = bits.read(16)? + 1;
    let height = bits.read(16)? + 1;
    let (display_width, display_height) = if bits.read(1)? != 0 {
        (bits.read(16)? + 1, bits.read(16)? + 1)
    } else {
        (width, height)
    };
    Ok(Dimensions {
        width,
        height,
        display_width,
        display_height,
    })
}

struct Bits<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl Bits<'_> {
    fn read(&mut self, count: usize) -> Result<u32, String> {
        let mut value = 0;
        for _ in 0..count {
            let byte = self
                .bytes
                .get(self.offset / 8)
                .ok_or("Неполный заголовок VP9")?;
            value = (value << 1) | u32::from((byte >> (7 - self.offset % 8)) & 1);
            self.offset += 1;
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests;
