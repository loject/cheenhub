//! Нативные контексты FFmpeg: QSV/VAAPI и программный VP9 через libavcodec.

use crate::check;
use ffmpeg_next::{encoder, ffi, frame};
use std::ptr;

/// Пара согласованных FFmpeg encoder, аппаратного устройства и формата кадров.
#[derive(Clone, Copy)]
pub(crate) struct Candidate {
    /// Имя encoder для поиска в статической сборке и диагностики отказа.
    pub(crate) name: &'static str,
    device: Option<ffi::AVHWDeviceType>,
    pixel: ffi::AVPixelFormat,
}

/// Возвращает аппаратные варианты текущей платформы, затем программный резерв.
pub(crate) fn candidates() -> &'static [Candidate] {
    #[cfg(target_os = "windows")]
    const CANDIDATES: &[Candidate] = &[
        Candidate {
            name: "vp9_qsv",
            device: Some(ffi::AVHWDeviceType::AV_HWDEVICE_TYPE_QSV),
            pixel: ffi::AVPixelFormat::AV_PIX_FMT_QSV,
        },
        SOFTWARE,
    ];
    #[cfg(target_os = "linux")]
    const CANDIDATES: &[Candidate] = &[
        Candidate {
            name: "vp9_vaapi",
            device: Some(ffi::AVHWDeviceType::AV_HWDEVICE_TYPE_VAAPI),
            pixel: ffi::AVPixelFormat::AV_PIX_FMT_VAAPI,
        },
        Candidate {
            name: "vp9_qsv",
            device: Some(ffi::AVHWDeviceType::AV_HWDEVICE_TYPE_QSV),
            pixel: ffi::AVPixelFormat::AV_PIX_FMT_QSV,
        },
        SOFTWARE,
    ];
    CANDIDATES
}

const SOFTWARE: Candidate = Candidate {
    name: "libvpx-vp9",
    device: None,
    pixel: ffi::AVPixelFormat::AV_PIX_FMT_YUV420P,
};

impl Candidate {
    /// Показывает, нужен ли upload NV12 в аппаратное устройство этого encoder.
    pub(crate) fn is_hardware(&self) -> bool {
        self.device.is_some()
    }

    /// Возвращает CPU-формат входного кадра до необязательного аппаратного upload.
    pub(crate) fn input_format(&self) -> ffmpeg_next::format::Pixel {
        if self.is_hardware() {
            ffmpeg_next::format::Pixel::NV12
        } else {
            ffmpeg_next::format::Pixel::YUV420P
        }
    }
}

struct Buffer(*mut ffi::AVBufferRef);

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe { ffi::av_buffer_unref(&mut self.0) };
    }
}

/// Прикрепляет устройство и пул к контексту, который владеет их ссылками.
pub(crate) fn configure(
    encoder: &mut encoder::video::Video,
    candidate: &Candidate,
) -> Result<(), String> {
    let Some(device_type) = candidate.device else {
        encoder.set_format(candidate.input_format());
        return Ok(());
    };
    let mut device = Buffer(ptr::null_mut());
    unsafe {
        check(ffi::av_hwdevice_ctx_create(
            &mut device.0,
            device_type,
            ptr::null(),
            ptr::null_mut(),
            0,
        ))?;
        let frames = Buffer(ffi::av_hwframe_ctx_alloc(device.0));
        if frames.0.is_null() {
            return Err("Не удалось создать пул аппаратных кадров".into());
        }
        let pool = &mut *((*frames.0).data.cast::<ffi::AVHWFramesContext>());
        pool.format = candidate.pixel;
        pool.sw_format = ffi::AVPixelFormat::AV_PIX_FMT_NV12;
        pool.width = ((encoder.width() + 31) & !31) as i32;
        pool.height = ((encoder.height() + 31) & !31) as i32;
        pool.initial_pool_size = 8;
        check(ffi::av_hwframe_ctx_init(frames.0))?;
        let context = &mut *encoder.as_mut_ptr();
        context.hw_frames_ctx = ffi::av_buffer_ref(frames.0);
        context.hw_device_ctx = ffi::av_buffer_ref(device.0);
        if context.hw_frames_ctx.is_null() || context.hw_device_ctx.is_null() {
            return Err("Не удалось сохранить аппаратный контекст".into());
        }
        context.pix_fmt = candidate.pixel;
    }
    Ok(())
}

/// Загружает NV12 в новый аппаратный кадр, принадлежащий вызывающему encoder.
pub(crate) fn upload(
    encoder: &encoder::video::Encoder,
    software: &frame::Video,
) -> Result<frame::Video, String> {
    let mut hardware = frame::Video::empty();
    unsafe {
        check(ffi::av_hwframe_get_buffer(
            (*encoder.as_ptr()).hw_frames_ctx,
            hardware.as_mut_ptr(),
            0,
        ))?;
        check(ffi::av_hwframe_transfer_data(
            hardware.as_mut_ptr(),
            software.as_ptr(),
            0,
        ))?;
    }
    hardware.set_width(software.width());
    hardware.set_height(software.height());
    hardware.set_pts(software.pts());
    hardware.set_kind(software.kind());
    Ok(hardware)
}
