//! Android VP9-кодирование через системный MediaCodec.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use futures_util::future::{LocalBoxFuture, ready};
use jni::objects::{GlobalRef, JByteBuffer, JObject, JValue};
use jni::{JNIEnv, JavaVM};
use ndk_context::android_context;

use super::super::backend::{
    EncodedVideoFrame, EncodedVideoFrameCallback, VideoCodec, VideoEncoderConfig,
    VideoEncoderDescriptor, VideoEncodingAcceleratorKind, VideoEncodingError, VideoEncodingManager,
    VideoFrameEncoder,
};

const MIME_VP9: &str = "video/x-vnd.on2.vp9";
const CONFIGURE_FLAG_ENCODE: i32 = 1;
const BUFFER_FLAG_KEY_FRAME: i32 = 1;
const BUFFER_FLAG_CODEC_CONFIG: i32 = 2;

/// Ссылка на входной `Surface` Android-кодировщика.
#[derive(Clone)]
pub(crate) struct AndroidEncoderSurface(GlobalRef);

impl AndroidEncoderSurface {
    /// Возвращает JNI-объект для передачи Camera2 или MediaProjection.
    pub(crate) fn as_obj(&self) -> &JObject<'_> {
        self.0.as_obj()
    }
}

/// VP9-кодировщик с Surface-входом, принадлежащий Android `MediaCodec`.
pub(crate) struct AndroidSurfaceVideoEncoder {
    vm: JavaVM,
    codec: GlobalRef,
    surface: AndroidEncoderSurface,
    config: VideoEncoderConfig,
    callback: EncodedVideoFrameCallback,
    sequence: Cell<u64>,
    closed: Cell<bool>,
    buffer_info: RefCell<GlobalRef>,
}

impl AndroidSurfaceVideoEncoder {
    fn create(
        config: VideoEncoderConfig,
        callback: EncodedVideoFrameCallback,
    ) -> Result<Self, VideoEncodingError> {
        if config.width == 0 || config.height == 0 || config.frame_rate == 0 {
            return Err(VideoEncodingError::unavailable(
                "Некорректная конфигурация Android VP9 encoder",
            ));
        }
        let context = android_context();
        let vm = unsafe { JavaVM::from_raw(context.vm().cast()) }
            .map_err(|error| media_error("Не удалось получить Android JavaVM", error))?;
        let mut env = vm
            .attach_current_thread()
            .map_err(|error| media_error("Не удалось подключить поток к Android JavaVM", error))?;

        let mime = env
            .new_string(MIME_VP9)
            .map_err(|error| media_error("Не удалось создать MIME VP9", error))?;
        let codec = env
            .call_static_method(
                "android/media/MediaCodec",
                "createEncoderByType",
                "(Ljava/lang/String;)Landroid/media/MediaCodec;",
                &[JValue::Object(&mime)],
            )
            .and_then(|value| value.l())
            .map_err(|error| {
                media_error("Устройство не предоставляет VP9 MediaCodec encoder", error)
            })?;

        let format = env
            .call_static_method(
                "android/media/MediaFormat",
                "createVideoFormat",
                "(Ljava/lang/String;II)Landroid/media/MediaFormat;",
                &[
                    JValue::Object(&mime),
                    JValue::Int(config.width as i32),
                    JValue::Int(config.height as i32),
                ],
            )
            .and_then(|value| value.l())
            .map_err(|error| media_error("Не удалось создать MediaFormat для VP9", error))?;
        set_integer(&mut env, &format, "bitrate", config.bitrate_bps as i32)?;
        set_integer(&mut env, &format, "frame-rate", config.frame_rate as i32)?;
        set_integer(&mut env, &format, "i-frame-interval", 2)?;
        set_integer(&mut env, &format, "color-format", 0x7F00_0789)?; // COLOR_FormatSurface

        env.call_method(
            &codec,
            "configure",
            "(Landroid/media/MediaFormat;Landroid/view/Surface;Landroid/media/MediaCrypto;I)V",
            &[
                JValue::Object(&format),
                JValue::Object(&JObject::null()),
                JValue::Object(&JObject::null()),
                JValue::Int(CONFIGURE_FLAG_ENCODE),
            ],
        )
        .map_err(|error| media_error("VP9 MediaCodec отклонил конфигурацию", error))?;
        let surface = env
            .call_method(
                &codec,
                "createInputSurface",
                "()Landroid/view/Surface;",
                &[],
            )
            .and_then(|value| value.l())
            .and_then(|surface| env.new_global_ref(surface))
            .map_err(|error| {
                media_error("Не удалось создать входной Surface VP9 encoder", error)
            })?;
        env.call_method(&codec, "start", "()V", &[])
            .map_err(|error| media_error("Не удалось запустить VP9 MediaCodec", error))?;
        let buffer_info = env
            .new_object("android/media/MediaCodec$BufferInfo", "()V", &[])
            .and_then(|info| env.new_global_ref(info))
            .map_err(|error| media_error("Не удалось создать MediaCodec.BufferInfo", error))?;
        let codec = env
            .new_global_ref(codec)
            .map_err(|error| media_error("Не удалось сохранить VP9 MediaCodec", error))?;
        drop(env);

        Ok(Self {
            vm,
            codec,
            surface: AndroidEncoderSurface(surface),
            config,
            callback,
            sequence: Cell::new(0),
            closed: Cell::new(false),
            buffer_info: RefCell::new(buffer_info),
        })
    }

    /// Возвращает Surface, в который источник должен записывать кадры.
    pub(crate) fn input_surface(&self) -> AndroidEncoderSurface {
        self.surface.clone()
    }

    /// Извлекает все готовые encoded buffers без блокировки вызывающего потока.
    pub(crate) fn drain(&self) -> Result<(), VideoEncodingError> {
        if self.closed.get() {
            return Ok(());
        }
        let mut env = self
            .vm
            .attach_current_thread()
            .map_err(|error| media_error("Не удалось подключить поток VP9 drain", error))?;
        loop {
            let info = self.buffer_info.borrow();
            let index = env
                .call_method(
                    self.codec.as_obj(),
                    "dequeueOutputBuffer",
                    "(Landroid/media/MediaCodec$BufferInfo;J)I",
                    &[JValue::Object(info.as_obj()), JValue::Long(0)],
                )
                .and_then(|value| value.i())
                .map_err(|error| media_error("Ошибка чтения VP9 MediaCodec output", error))?;
            if index < 0 {
                break;
            }
            let flags = env
                .get_field(info.as_obj(), "flags", "I")
                .and_then(|v| v.i())
                .map_err(|error| media_error("Не удалось прочитать flags encoded buffer", error))?;
            let size = env
                .get_field(info.as_obj(), "size", "I")
                .and_then(|v| v.i())
                .map_err(|error| {
                    media_error("Не удалось прочитать размер encoded buffer", error)
                })?;
            let offset = env
                .get_field(info.as_obj(), "offset", "I")
                .and_then(|value| value.i())
                .map_err(|error| {
                    media_error("Не удалось прочитать смещение encoded buffer", error)
                })?;
            let timestamp_us = env
                .get_field(info.as_obj(), "presentationTimeUs", "J")
                .and_then(|v| v.j())
                .map_err(|error| {
                    media_error("Не удалось прочитать timestamp encoded buffer", error)
                })?;
            if size > 0 && flags & BUFFER_FLAG_CODEC_CONFIG == 0 {
                let buffer = env
                    .call_method(
                        self.codec.as_obj(),
                        "getOutputBuffer",
                        "(I)Ljava/nio/ByteBuffer;",
                        &[JValue::Int(index)],
                    )
                    .and_then(|value| value.l())
                    .map_err(|error| media_error("Не удалось получить VP9 output buffer", error))?;
                let byte_buffer = JByteBuffer::from(buffer);
                let address = env
                    .get_direct_buffer_address(&byte_buffer)
                    .map_err(|error| {
                        media_error("VP9 output buffer не является direct ByteBuffer", error)
                    })?;
                let capacity = env
                    .get_direct_buffer_capacity(&byte_buffer)
                    .map_err(|error| {
                        media_error("Не удалось получить размер VP9 output buffer", error)
                    })?;
                let start = offset.max(0) as usize;
                let end = start.saturating_add(size as usize);
                if end > capacity {
                    return Err(VideoEncodingError::unavailable(
                        "MediaCodec вернул encoded buffer за пределами ByteBuffer",
                    ));
                }
                // Указатель принадлежит direct ByteBuffer и остаётся действительным до
                // releaseOutputBuffer ниже; границы предварительно сверены с capacity.
                let bytes =
                    unsafe { std::slice::from_raw_parts(address.add(start), size as usize) }
                        .to_vec();
                let sequence = self.sequence.get();
                self.sequence.set(sequence.wrapping_add(1));
                (self.callback)(EncodedVideoFrame {
                    sequence,
                    timestamp_us: timestamp_us.max(0) as u64,
                    duration_us: 1_000_000 / self.config.frame_rate,
                    codec: VideoCodec::Vp9,
                    key_frame: flags & BUFFER_FLAG_KEY_FRAME != 0,
                    width: self.config.width,
                    height: self.config.height,
                    bytes,
                });
            }
            env.call_method(
                self.codec.as_obj(),
                "releaseOutputBuffer",
                "(IZ)V",
                &[JValue::Int(index), JValue::Bool(0)],
            )
            .map_err(|error| media_error("Не удалось освободить VP9 output buffer", error))?;
        }
        Ok(())
    }
}

impl VideoFrameEncoder for AndroidSurfaceVideoEncoder {
    type InputFrame = ();

    fn encode(&self, _frame: &(), _key_frame: bool) -> Result<(), VideoEncodingError> {
        self.drain()
    }

    fn close(&self) -> Result<(), VideoEncodingError> {
        if self.closed.replace(true) {
            return Ok(());
        }
        let mut env = self.vm.attach_current_thread().map_err(|error| {
            media_error("Не удалось подключить поток остановки VP9 encoder", error)
        })?;
        let _ = env.call_method(self.codec.as_obj(), "signalEndOfInputStream", "()V", &[]);
        env.call_method(self.codec.as_obj(), "stop", "()V", &[])
            .map_err(|error| media_error("Не удалось остановить VP9 MediaCodec", error))?;
        env.call_method(self.codec.as_obj(), "release", "()V", &[])
            .map_err(|error| media_error("Не удалось освободить VP9 MediaCodec", error))?;
        Ok(())
    }
}

impl Drop for AndroidSurfaceVideoEncoder {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

/// Менеджер аппаратного Android-кодировщика.
pub(crate) struct AndroidVideoEncodingManager;

impl VideoEncodingManager for AndroidVideoEncodingManager {
    type InputFrame = ();
    type Encoder = AndroidSurfaceVideoEncoder;

    fn available_accelerators(
        &self,
        config: VideoEncoderConfig,
    ) -> LocalBoxFuture<'static, Result<Vec<VideoEncoderDescriptor>, VideoEncodingError>> {
        Box::pin(ready(probe(config).map(|available| {
            if available {
                vec![descriptor()]
            } else {
                vec![]
            }
        })))
    }

    fn create_encoder(
        &self,
        kind: VideoEncodingAcceleratorKind,
        config: VideoEncoderConfig,
        callback: EncodedVideoFrameCallback,
    ) -> LocalBoxFuture<'static, Result<Self::Encoder, VideoEncodingError>> {
        Box::pin(ready(if kind == VideoEncodingAcceleratorKind::Native {
            AndroidSurfaceVideoEncoder::create(config, callback)
        } else {
            Err(VideoEncodingError::unsupported(
                "На Android доступен только системный MediaCodec encoder",
            ))
        }))
    }
}

fn descriptor() -> VideoEncoderDescriptor {
    VideoEncoderDescriptor {
        id: "android-mediacodec-vp9".into(),
        label: "Android MediaCodec VP9".into(),
        kind: VideoEncodingAcceleratorKind::Native,
        codecs: vec![VideoCodec::Vp9],
    }
}

fn probe(config: VideoEncoderConfig) -> Result<bool, VideoEncodingError> {
    match AndroidSurfaceVideoEncoder::create(config, Rc::new(|_| {})) {
        Ok(encoder) => {
            encoder.close()?;
            Ok(true)
        }
        Err(error) if error.is_unsupported() => Ok(false),
        Err(_) => Ok(false),
    }
}

fn set_integer(
    env: &mut JNIEnv<'_>,
    format: &JObject<'_>,
    key: &str,
    value: i32,
) -> Result<(), VideoEncodingError> {
    let key = env
        .new_string(key)
        .map_err(|error| media_error("Не удалось создать ключ MediaFormat", error))?;
    env.call_method(
        format,
        "setInteger",
        "(Ljava/lang/String;I)V",
        &[JValue::Object(&key), JValue::Int(value)],
    )
    .map_err(|error| media_error("Не удалось настроить MediaFormat", error))?;
    Ok(())
}

fn media_error(context: &str, error: impl std::fmt::Display) -> VideoEncodingError {
    VideoEncodingError::unavailable(format!("{context}: {error}"))
}
