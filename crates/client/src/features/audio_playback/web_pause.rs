//! Доигрывание остатка browser PCM после паузы голосового потока.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use dioxus::prelude::{debug, warn};
use wasm_bindgen_futures::spawn_local;
use web_sys::AudioContext;

use super::AudioPlaybackInner;
use super::browser_helpers::js_error_message;
use super::jitter_runtime::jitter_now_us;
use super::playback_schedule::schedule_pcm;
use crate::features::runtime::sleep_duration;

/// Продлевает ожидание хвоста после декодирования и запускает одну задачу на отправителя.
pub(super) fn arm_playback_pause(
    context: &AudioContext,
    inner: &Rc<RefCell<AudioPlaybackInner>>,
    sender_user_id: &str,
    duration_us: u32,
    sample_rate: f32,
) {
    let pause = {
        let inner = inner.borrow();
        let Some(sender) = inner.senders.get(sender_user_id) else {
            return;
        };
        if !sender
            .pause
            .refresh(jitter_now_us(), duration_us, inner.jitter_buffer_us)
        {
            return;
        }
        sender.pause.clone()
    };
    let context = context.clone();
    let inner = inner.clone();
    let sender_user_id = sender_user_id.to_owned();
    spawn_local(async move {
        while let Some(wait_us) = pause.remaining_us(jitter_now_us()) {
            if wait_us > 0 {
                sleep_duration(Duration::from_micros(wait_us)).await;
                continue;
            }
            if pause.take_due(jitter_now_us()) {
                flush_playback_tail(&context, &inner, &sender_user_id, sample_rate);
            }
            break;
        }
    });
}

fn flush_playback_tail(
    context: &AudioContext,
    inner: &Rc<RefCell<AudioPlaybackInner>>,
    sender_user_id: &str,
    sample_rate: f32,
) {
    let planes = {
        let inner = inner.borrow();
        if inner.muted {
            return;
        }
        let Some(sender) = inner.senders.get(sender_user_id) else {
            return;
        };
        sender
            .time_compressors
            .borrow()
            .iter()
            .map(|compressor| compressor.borrow_mut().flush())
            .collect::<Vec<_>>()
    };
    let tail_samples = planes.first().map_or(0, Vec::len);
    if tail_samples == 0 {
        return;
    }
    debug!(%sender_user_id, tail_samples, "flushing browser voice tail after pause");
    if let Err(error) = schedule_pcm(context, inner, sender_user_id, &planes, sample_rate, false) {
        warn!(%sender_user_id, error = %js_error_message(error), "failed to schedule browser voice tail");
    }
}
