//! Windows-реализация декодирования VP9 видеопотоков участников.

use std::cell::Cell;
use std::rc::Rc;

use base64::Engine;
use dioxus::document::{self, Eval};
use dioxus::logger::tracing::{debug, info, warn};
use dioxus::prelude::spawn;
use futures_util::future::LocalBoxFuture;

use super::super::ParticipantVideoFrame;
use super::super::backend::{
    ParticipantVideoBackend, ParticipantVideoRenderError, ParticipantVideoRenderer,
};

/// Backend декодирования видео участников для Windows.
pub(crate) struct WindowsParticipantVideoBackend;

impl ParticipantVideoBackend for WindowsParticipantVideoBackend {
    fn create_renderer(
        &self,
        target_id: String,
        user_id: String,
        source_label: &'static str,
    ) -> Result<Rc<dyn ParticipantVideoRenderer>, ParticipantVideoRenderError> {
        WindowsParticipantVideoRenderer::new(target_id, user_id, source_label)
            .map(|renderer| Rc::new(renderer) as Rc<dyn ParticipantVideoRenderer>)
    }
}

struct WindowsParticipantVideoRenderer {
    canvas_eval: Eval,
    canvas_failed: Rc<Cell<bool>>,
    user_id: String,
    source_label: &'static str,
    closed: Cell<bool>,
    received_key_frame: Cell<bool>,
    waiting_key_frame_logged: Cell<bool>,
}

#[derive(serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum CanvasMessage<'a> {
    Frame {
        timestamp_us: u64,
        duration_us: u32,
        key_frame: bool,
        vp9_base64: &'a str,
    },
    Close,
}

const CANVAS_BRIDGE_SCRIPT: &str = r#"
let canvas = null;
let decoder = null;
try {
    const targetId = await dioxus.recv();
    let target = document.getElementById(targetId);
    for (let attempt = 0; !target && attempt < 60; attempt++) {
        await new Promise(resolve => requestAnimationFrame(resolve));
        target = document.getElementById(targetId);
    }
    if (!target) throw new Error("Контейнер видео участника не найден.");
    canvas = document.createElement("canvas");
    canvas.className = "absolute inset-0 z-0 h-full w-full object-contain";
    canvas.setAttribute("aria-hidden", "true");
    target.appendChild(canvas);
    const context = canvas.getContext("2d", { alpha: false });
    if (!context) throw new Error("Не удалось создать 2D-контекст canvas.");
    if (typeof VideoDecoder === "undefined" || typeof EncodedVideoChunk === "undefined") {
        throw new Error("WebView не поддерживает VP9-декодирование WebCodecs.");
    }
    const config = { codec: "vp09.00.10.08" };
    const support = await VideoDecoder.isConfigSupported(config);
    if (!support.supported) throw new Error("WebView не поддерживает VP9 WebCodecs.");
    decoder = new VideoDecoder({
        output: frame => {
            try {
                const width = frame.displayWidth || frame.codedWidth;
                const height = frame.displayHeight || frame.codedHeight;
                if (canvas.width !== width) canvas.width = width;
                if (canvas.height !== height) canvas.height = height;
                context.drawImage(frame, 0, 0, width, height);
            } catch (error) {
                void dioxus.send(String(error)).catch(() => {});
            } finally {
                frame.close();
            }
        },
        error: error => {
            void dioxus.send(String(error)).catch(() => {});
        },
    });
    decoder.configure(config);
    while (true) {
        const message = await dioxus.recv();
        if (message.kind === "close") {
            decoder.close();
            decoder = null;
            return true;
        }
        try {
            const binary = atob(message.vp9_base64);
            const bytes = new Uint8Array(binary.length);
            for (let index = 0; index < binary.length; index++) {
                bytes[index] = binary.charCodeAt(index);
            }
            decoder.decode(new EncodedVideoChunk({
                type: message.key_frame ? "key" : "delta",
                timestamp: message.timestamp_us,
                duration: message.duration_us,
                data: bytes,
            }));
        } catch (error) {
            try {
                await dioxus.send(String(error));
            } catch (_) {}
            return false;
        }
    }
} catch (error) {
    try {
        await dioxus.send(String(error));
    } catch (_) {}
    return false;
} finally {
    if (decoder && decoder.state !== "closed") decoder.close();
    if (canvas) canvas.remove();
}
"#;

impl WindowsParticipantVideoRenderer {
    fn new(
        target_id: String,
        user_id: String,
        source_label: &'static str,
    ) -> Result<Self, ParticipantVideoRenderError> {
        let canvas_eval = document::eval(CANVAS_BRIDGE_SCRIPT);
        canvas_eval.send(&target_id).map_err(|error| {
            ParticipantVideoRenderError::new(format!(
                "Не удалось запустить canvas для видео участника: {error}"
            ))
        })?;
        let canvas_failed = Rc::new(Cell::new(false));
        let canvas_lifecycle = canvas_eval;
        let lifecycle_user_id = user_id.clone();
        let lifecycle_failed = canvas_failed.clone();
        spawn(async move {
            match canvas_lifecycle.join::<bool>().await {
                Ok(true) => {}
                Ok(false) => {
                    if !lifecycle_failed.replace(true) {
                        warn!(
                            sender_user_id = %lifecycle_user_id,
                            source = source_label,
                            "canvas WebView не смог запустить отрисовку"
                        );
                    }
                }
                Err(error) => {
                    if !lifecycle_failed.replace(true) {
                        warn!(
                            sender_user_id = %lifecycle_user_id,
                            source = source_label,
                            %error,
                            "canvas WebView завершился с ошибкой"
                        );
                    }
                }
            }
        });
        let mut canvas_errors = canvas_eval;
        let error_user_id = user_id.clone();
        let error_failed = canvas_failed.clone();
        spawn(async move {
            while let Ok(error) = canvas_errors.recv::<String>().await {
                if !error_failed.replace(true) {
                    warn!(
                        sender_user_id = %error_user_id,
                        source = source_label,
                        %error,
                        "ошибка отрисовки кадра в canvas WebView"
                    );
                }
            }
        });

        info!(
            sender_user_id = %user_id,
            source = source_label,
            "создан WebCodecs renderer видео участника в Windows"
        );
        Ok(Self {
            canvas_eval,
            canvas_failed,
            user_id,
            source_label,
            closed: Cell::new(false),
            received_key_frame: Cell::new(false),
            waiting_key_frame_logged: Cell::new(false),
        })
    }
}

impl ParticipantVideoRenderer for WindowsParticipantVideoRenderer {
    fn decode(
        &self,
        frame: ParticipantVideoFrame,
    ) -> LocalBoxFuture<'_, Result<(), ParticipantVideoRenderError>> {
        if self.closed.get() || self.canvas_failed.get() || frame.bytes.is_empty() {
            return Box::pin(async { Ok(()) });
        }
        if !self.received_key_frame.get() && !frame.key_frame {
            if !self.waiting_key_frame_logged.replace(true) {
                debug!(
                    sender_user_id = %self.user_id,
                    sequence = frame.sequence,
                    source = self.source_label,
                    "ожидание ключевого VP9-кадра участника перед декодированием"
                );
            }
            return Box::pin(async { Ok(()) });
        }
        if frame.key_frame && !self.received_key_frame.replace(true) {
            info!(
                sender_user_id = %self.user_id,
                sequence = frame.sequence,
                source = self.source_label,
                "получен первый ключевой VP9-кадр участника в Windows"
            );
        }
        let vp9_base64 = base64::engine::general_purpose::STANDARD.encode(frame.bytes);
        Box::pin(async move {
            self.canvas_eval
                .send(CanvasMessage::Frame {
                    timestamp_us: frame.timestamp_us,
                    duration_us: frame.duration_us,
                    key_frame: frame.key_frame,
                    vp9_base64: &vp9_base64,
                })
                .map_err(|error| {
                    self.canvas_failed.set(true);
                    ParticipantVideoRenderError::new(format!(
                        "Не удалось передать VP9-кадр в WebView: {error}"
                    ))
                })?;
            Ok(())
        })
    }

    fn close(&self) {
        if self.closed.replace(true) {
            return;
        }
        if let Err(error) = self.canvas_eval.send(CanvasMessage::Close) {
            debug!(
                sender_user_id = %self.user_id,
                source = self.source_label,
                %error,
                "не удалось отправить команду закрытия canvas WebView"
            );
        }
        debug!(
            sender_user_id = %self.user_id,
            source = self.source_label,
            "закрыт renderer видео участника в Windows"
        );
    }
}

impl Drop for WindowsParticipantVideoRenderer {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::CanvasMessage;

    #[test]
    fn canvas_frame_message_preserves_vp9_timing_and_key_frame_metadata() {
        let message = CanvasMessage::Frame {
            timestamp_us: 12_345,
            duration_us: 33_333,
            key_frame: true,
            vp9_base64: "AQID",
        };
        let value = serde_json::to_value(message).expect("canvas frame serializes");

        assert_eq!(value["kind"], "frame");
        assert_eq!(value["timestamp_us"], 12_345);
        assert_eq!(value["duration_us"], 33_333);
        assert_eq!(value["key_frame"], true);
        assert_eq!(value["vp9_base64"], "AQID");
        assert!(value.get("rgba_base64").is_none());
    }
}
