//! Android VP9-кодирование через системный MediaCodec.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use futures_util::future::{LocalBoxFuture, ready};
use jni::objects::{GlobalRef, JByteBuffer, JObject, JValue};
use jni::{JNIEnv, JavaVM};
use ndk_context::android_context;

use crate::features::runtime::android::guard_jni_result;

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

        let mime = guard_media_jni(
            &mut env,
            "MediaCodec.mime.new_string",
            "Не удалось создать MIME VP9",
            |env| env.new_string(MIME_VP9),
        )?;
        let codec = guard_media_jni(
            &mut env,
            "MediaCodec.createEncoderByType",
            "Устройство не предоставляет VP9 MediaCodec encoder",
            |env| {
                env.call_static_method(
                    "android/media/MediaCodec",
                    "createEncoderByType",
                    "(Ljava/lang/String;)Landroid/media/MediaCodec;",
                    &[JValue::Object(&mime)],
                )
            },
        )?
        .l()
        .map_err(|error| {
            media_error("Устройство не предоставляет VP9 MediaCodec encoder", error)
        })?;

        let format = guard_media_jni(
            &mut env,
            "MediaFormat.createVideoFormat",
            "Не удалось создать MediaFormat для VP9",
            |env| {
                env.call_static_method(
                    "android/media/MediaFormat",
                    "createVideoFormat",
                    "(Ljava/lang/String;II)Landroid/media/MediaFormat;",
                    &[
                        JValue::Object(&mime),
                        JValue::Int(config.width as i32),
                        JValue::Int(config.height as i32),
                    ],
                )
            },
        )?
        .l()
        .map_err(|error| media_error("Не удалось создать MediaFormat для VP9", error))?;
        set_integer(&mut env, &format, "bitrate", config.bitrate_bps as i32)?;
        set_integer(&mut env, &format, "frame-rate", config.frame_rate as i32)?;
        set_integer(&mut env, &format, "i-frame-interval", 2)?;
        set_integer(&mut env, &format, "color-format", 0x7F00_0789)?; // COLOR_FormatSurface

        guard_media_jni(
            &mut env,
            "MediaCodec.configure",
            "VP9 MediaCodec отклонил конфигурацию",
            |env| {
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
            },
        )?;
        let surface = guard_media_jni(
            &mut env,
            "MediaCodec.createInputSurface",
            "Не удалось создать входной Surface VP9 encoder",
            |env| {
                env.call_method(
                    &codec,
                    "createInputSurface",
                    "()Landroid/view/Surface;",
                    &[],
                )
            },
        )?
        .l()
        .map_err(|error| media_error("Не удалось создать входной Surface VP9 encoder", error))?;
        let surface = guard_media_jni(
            &mut env,
            "MediaCodec.createInputSurface.new_global_ref",
            "Не удалось сохранить входной Surface VP9 encoder",
            |env| env.new_global_ref(&surface),
        )?;
        guard_media_jni(
            &mut env,
            "MediaCodec.start",
            "Не удалось запустить VP9 MediaCodec",
            |env| env.call_method(&codec, "start", "()V", &[]),
        )?;
        let buffer_info = guard_media_jni(
            &mut env,
            "MediaCodec.BufferInfo.new_object",
            "Не удалось создать MediaCodec.BufferInfo",
            |env| env.new_object("android/media/MediaCodec$BufferInfo", "()V", &[]),
        )?;
        let buffer_info = guard_media_jni(
            &mut env,
            "MediaCodec.BufferInfo.new_global_ref",
            "Не удалось сохранить MediaCodec.BufferInfo",
            |env| env.new_global_ref(&buffer_info),
        )?;
        let codec = guard_media_jni(
            &mut env,
            "MediaCodec.new_global_ref",
            "Не удалось сохранить VP9 MediaCodec",
            |env| env.new_global_ref(&codec),
        )?;
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
            let index = guard_media_jni(
                &mut env,
                "MediaCodec.dequeueOutputBuffer",
                "Ошибка чтения VP9 MediaCodec output",
                |env| {
                    env.call_method(
                        self.codec.as_obj(),
                        "dequeueOutputBuffer",
                        "(Landroid/media/MediaCodec$BufferInfo;J)I",
                        &[JValue::Object(info.as_obj()), JValue::Long(0)],
                    )
                },
            )?
            .i()
            .map_err(|error| media_error("Ошибка чтения VP9 MediaCodec output", error))?;
            if index < 0 {
                break;
            }
            let flags = guard_media_jni(
                &mut env,
                "MediaCodec.BufferInfo.flags",
                "Не удалось прочитать flags encoded buffer",
                |env| env.get_field(info.as_obj(), "flags", "I"),
            )?
            .i()
            .map_err(|error| media_error("Не удалось прочитать flags encoded buffer", error))?;
            let size = guard_media_jni(
                &mut env,
                "MediaCodec.BufferInfo.size",
                "Не удалось прочитать размер encoded buffer",
                |env| env.get_field(info.as_obj(), "size", "I"),
            )?
            .i()
            .map_err(|error| media_error("Не удалось прочитать размер encoded buffer", error))?;
            let offset = guard_media_jni(
                &mut env,
                "MediaCodec.BufferInfo.offset",
                "Не удалось прочитать смещение encoded buffer",
                |env| env.get_field(info.as_obj(), "offset", "I"),
            )?
            .i()
            .map_err(|error| media_error("Не удалось прочитать смещение encoded buffer", error))?;
            let timestamp_us = guard_media_jni(
                &mut env,
                "MediaCodec.BufferInfo.presentationTimeUs",
                "Не удалось прочитать timestamp encoded buffer",
                |env| env.get_field(info.as_obj(), "presentationTimeUs", "J"),
            )?
            .j()
            .map_err(|error| media_error("Не удалось прочитать timestamp encoded buffer", error))?;
            if size > 0 && flags & BUFFER_FLAG_CODEC_CONFIG == 0 {
                let buffer = guard_media_jni(
                    &mut env,
                    "MediaCodec.getOutputBuffer",
                    "Не удалось получить VP9 output buffer",
                    |env| {
                        env.call_method(
                            self.codec.as_obj(),
                            "getOutputBuffer",
                            "(I)Ljava/nio/ByteBuffer;",
                            &[JValue::Int(index)],
                        )
                    },
                )?
                .l()
                .map_err(|error| media_error("Не удалось получить VP9 output buffer", error))?;
                let byte_buffer = JByteBuffer::from(buffer);
                let address = guard_media_jni(
                    &mut env,
                    "ByteBuffer.get_direct_buffer_address",
                    "VP9 output buffer не является direct ByteBuffer",
                    |env| env.get_direct_buffer_address(&byte_buffer),
                )?;
                let capacity = guard_media_jni(
                    &mut env,
                    "ByteBuffer.get_direct_buffer_capacity",
                    "Не удалось получить размер VP9 output buffer",
                    |env| env.get_direct_buffer_capacity(&byte_buffer),
                )?;
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
            guard_media_jni(
                &mut env,
                "MediaCodec.releaseOutputBuffer",
                "Не удалось освободить VP9 output buffer",
                |env| {
                    env.call_method(
                        self.codec.as_obj(),
                        "releaseOutputBuffer",
                        "(IZ)V",
                        &[JValue::Int(index), JValue::Bool(0)],
                    )
                },
            )?;
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
        let _ = guard_media_jni(
            &mut env,
            "MediaCodec.signalEndOfInputStream",
            "Не удалось завершить входной поток VP9 MediaCodec",
            |env| env.call_method(self.codec.as_obj(), "signalEndOfInputStream", "()V", &[]),
        );

        guard_media_jni(
            &mut env,
            "MediaCodec.stop",
            "Не удалось остановить VP9 MediaCodec",
            |env| env.call_method(self.codec.as_obj(), "stop", "()V", &[]),
        )?;

        guard_media_jni(
            &mut env,
            "MediaCodec.release",
            "Не удалось освободить VP9 MediaCodec",
            |env| env.call_method(self.codec.as_obj(), "release", "()V", &[]),
        )?;
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
    let key = guard_media_jni(
        env,
        "MediaFormat.setInteger.new_string",
        "Не удалось создать ключ MediaFormat",
        |env| env.new_string(key),
    )?;

    guard_media_jni(
        env,
        "MediaFormat.setInteger",
        "Не удалось настроить MediaFormat",
        |env| {
            env.call_method(
                format,
                "setInteger",
                "(Ljava/lang/String;I)V",
                &[JValue::Object(&key), JValue::Int(value)],
            )
        },
    )?;

    Ok(())
}

fn guard_media_jni<'local, T>(
    env: &mut JNIEnv<'local>,
    operation: &str,
    context: &str,
    call: impl FnOnce(&mut JNIEnv<'local>) -> jni::errors::Result<T>,
) -> Result<T, VideoEncodingError> {
    let result = call(env);
    guard_jni_result(env, operation, result).map_err(|error| media_error(context, error))
}

fn media_error(context: &str, error: impl std::fmt::Display) -> VideoEncodingError {
    VideoEncodingError::unavailable(format!("{context}: {error}"))
}
