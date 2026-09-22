//! Программное кодирование Windows-кадров через libvpx VP9.

use std::ffi::CStr;
use std::ptr;

use vpx_sys as vpx;

use crate::features::video_encoding::{EncodedVideoFrame, VideoCodec};

use super::frames::I420Frame;

#[link(name = "vpx", kind = "static")]
unsafe extern "C" {}

const KEY_FRAME_INTERVAL_US: u64 = 2_000_000;

/// Контекст VP9, принадлежащий одному потоку захвата.
pub(super) struct WindowsVp9Encoder {
    context: vpx::vpx_codec_ctx_t,
    width: u32,
    height: u32,
    frame_duration_us: u32,
    sequence: u64,
    last_key_frame_us: Option<u64>,
}

impl WindowsVp9Encoder {
    /// Создаёт кодировщик с точными размерами выбранного пресета.
    pub(super) fn new(width: u32, height: u32, bitrate_bps: u32, fps: u32) -> Result<Self, String> {
        if width == 0
            || height == 0
            || fps == 0
            || !width.is_multiple_of(2)
            || !height.is_multiple_of(2)
        {
            return Err("некорректное разрешение VP9".to_owned());
        }
        let mut configuration = std::mem::MaybeUninit::<vpx::vpx_codec_enc_cfg_t>::uninit();
        let interface = unsafe { vpx::vpx_codec_vp9_cx() };
        if interface.is_null() {
            return Err("libvpx не предоставляет VP9 encoder".to_owned());
        }
        check(unsafe {
            vpx::vpx_codec_enc_config_default(interface, configuration.as_mut_ptr(), 0)
        })?;
        // SAFETY: успешный вызов libvpx полностью заполнил конфигурацию.
        let mut configuration = unsafe { configuration.assume_init() };
        configuration.g_w = width;
        configuration.g_h = height;
        configuration.g_profile = 0;
        configuration.g_timebase.num = 1;
        configuration.g_timebase.den = 1_000_000;
        configuration.g_threads = 4;
        configuration.g_lag_in_frames = 0;
        configuration.g_error_resilient = vpx::VPX_ERROR_RESILIENT_DEFAULT;
        configuration.rc_end_usage = vpx::vpx_rc_mode::VPX_CBR;
        configuration.rc_target_bitrate = bitrate_bps.div_ceil(1_000);

        let mut context = std::mem::MaybeUninit::<vpx::vpx_codec_ctx_t>::uninit();
        check(unsafe {
            vpx::vpx_codec_enc_init_ver(
                context.as_mut_ptr(),
                interface,
                &configuration,
                0,
                vpx::VPX_ENCODER_ABI_VERSION as i32,
            )
        })?;
        // SAFETY: успешный вызов libvpx полностью заполнил контекст.
        let mut context = unsafe { context.assume_init() };
        if let Err(error) = check(unsafe {
            vpx::vpx_codec_control_(
                &mut context,
                vpx::vp8e_enc_control_id::VP8E_SET_CPUUSED as i32,
                7_i32,
            )
        }) {
            unsafe { vpx::vpx_codec_destroy(&mut context) };
            return Err(error);
        }
        if let Err(error) = check(unsafe {
            vpx::vpx_codec_control_(
                &mut context,
                vpx::vp8e_enc_control_id::VP9E_SET_ROW_MT as i32,
                1_i32,
            )
        }) {
            unsafe { vpx::vpx_codec_destroy(&mut context) };
            return Err(error);
        }
        Ok(Self {
            context,
            width,
            height,
            frame_duration_us: 1_000_000 / fps,
            sequence: 0,
            last_key_frame_us: None,
        })
    }

    /// Кодирует один I420-кадр и копирует выходные пакеты до следующего вызова libvpx.
    pub(super) fn encode(
        &mut self,
        frame: &I420Frame,
        timestamp_us: u64,
    ) -> Result<Vec<EncodedVideoFrame>, String> {
        let expected =
            usize::try_from(self.width).unwrap() * usize::try_from(self.height).unwrap() * 3 / 2;
        if frame.bytes.len() != expected {
            return Err("размер I420-кадра не совпадает с конфигурацией VP9".to_owned());
        }
        let mut image = std::mem::MaybeUninit::<vpx::vpx_image_t>::uninit();
        let wrapped = unsafe {
            vpx::vpx_img_wrap(
                image.as_mut_ptr(),
                vpx::vpx_img_fmt::VPX_IMG_FMT_I420,
                self.width,
                self.height,
                1,
                frame.bytes.as_ptr().cast_mut(),
            )
        };
        if wrapped.is_null() {
            return Err("libvpx не принял I420-кадр".to_owned());
        }
        let force_key_frame = should_force_key_frame(self.last_key_frame_us, timestamp_us);
        let flags = if force_key_frame {
            vpx::VPX_EFLAG_FORCE_KF as vpx::vpx_enc_frame_flags_t
        } else {
            0
        };
        let result = unsafe {
            vpx::vpx_codec_encode(
                &mut self.context,
                image.as_ptr(),
                timestamp_us.min(i64::MAX as u64) as i64,
                self.frame_duration_us as std::os::raw::c_ulong,
                flags,
                vpx::VPX_DL_REALTIME as std::os::raw::c_ulong,
            )
        };
        check(result)?;
        if force_key_frame {
            self.last_key_frame_us = Some(timestamp_us);
        }
        let mut output = Vec::new();
        let mut iterator: vpx::vpx_codec_iter_t = ptr::null();
        loop {
            let packet = unsafe { vpx::vpx_codec_get_cx_data(&mut self.context, &mut iterator) };
            if packet.is_null() {
                break;
            }
            let packet = unsafe { &*packet };
            if packet.kind != vpx::vpx_codec_cx_pkt_kind::VPX_CODEC_CX_FRAME_PKT {
                continue;
            }
            let encoded = unsafe { packet.data.frame };
            if encoded.buf.is_null() || encoded.sz == 0 {
                continue;
            }
            let bytes = unsafe { std::slice::from_raw_parts(encoded.buf.cast::<u8>(), encoded.sz) };
            output.push(EncodedVideoFrame {
                sequence: self.sequence,
                timestamp_us: encoded.pts.max(0) as u64,
                duration_us: encoded.duration.min(u32::MAX as _) as u32,
                codec: VideoCodec::Vp9,
                key_frame: encoded.flags & vpx::VPX_FRAME_IS_KEY != 0,
                width: self.width,
                height: self.height,
                bytes: bytes.to_vec(),
            });
            self.sequence = self.sequence.saturating_add(1);
        }
        Ok(output)
    }
}

impl Drop for WindowsVp9Encoder {
    fn drop(&mut self) {
        let result = unsafe { vpx::vpx_codec_destroy(&mut self.context) };
        if result != vpx::vpx_codec_err_t::VPX_CODEC_OK {
            dioxus::logger::tracing::warn!(?result, "failed to release libvpx encoder");
        }
    }
}

fn should_force_key_frame(last_key_frame_us: Option<u64>, timestamp_us: u64) -> bool {
    last_key_frame_us.is_none_or(|last| timestamp_us.saturating_sub(last) >= KEY_FRAME_INTERVAL_US)
}

fn check(result: vpx::vpx_codec_err_t) -> Result<(), String> {
    if result == vpx::vpx_codec_err_t::VPX_CODEC_OK {
        return Ok(());
    }
    let message = unsafe { CStr::from_ptr(vpx::vpx_codec_err_to_string(result)) };
    Err(format!("ошибка libvpx: {}", message.to_string_lossy()))
}

#[cfg(test)]
mod tests {
    use super::{WindowsVp9Encoder, should_force_key_frame};
    use crate::features::screen_share::native::windows::frames::I420Frame;

    #[test]
    fn first_frame_and_two_second_interval_request_key_frames() {
        assert!(should_force_key_frame(None, 0));
        assert!(!should_force_key_frame(Some(0), 1_999_999));
        assert!(should_force_key_frame(Some(0), 2_000_000));
    }

    #[test]
    fn encodes_i420_frame_with_vendored_libvpx() {
        let mut encoder = WindowsVp9Encoder::new(16, 16, 300_000, 15).unwrap();
        let mut bytes = vec![16; 16 * 16 * 3 / 2];
        bytes[16 * 16..].fill(128);
        let packets = encoder.encode(&I420Frame { bytes }, 0).unwrap();
        assert!(!packets.is_empty());
        assert!(packets[0].key_frame);
        assert_eq!(packets[0].duration_us, 1_000_000 / 15);
        assert_eq!(packets[0].width, 16);
        assert_eq!(packets[0].height, 16);
    }
}
