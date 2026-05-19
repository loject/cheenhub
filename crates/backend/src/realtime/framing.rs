//! Length-prefixed realtime stream framing.

use anyhow::{Context, anyhow};
use bytes::{BufMut, Bytes, BytesMut};
use cheenhub_contracts::realtime::RealtimeEnvelope;
use tokio::sync::Mutex;
use web_transport::{RecvStream, SendStream};

const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;

/// Writes one realtime JSON envelope frame to a reliable stream.
pub(crate) async fn write_envelope(
    send: &Mutex<SendStream>,
    envelope: &RealtimeEnvelope,
) -> anyhow::Result<()> {
    let bytes = serde_json::to_vec(envelope).context("failed to encode realtime envelope")?;
    write_frame(send, &bytes).await
}

/// Reads one realtime JSON envelope frame from a reliable stream.
pub(crate) async fn read_envelope(
    recv: &mut RecvStream,
) -> anyhow::Result<Option<RealtimeEnvelope>> {
    let Some(frame) = read_frame(recv).await? else {
        return Ok(None);
    };

    serde_json::from_slice(&frame)
        .map(Some)
        .context("failed to decode realtime envelope")
}

async fn write_frame(send: &Mutex<SendStream>, payload: &[u8]) -> anyhow::Result<()> {
    if payload.len() > MAX_FRAME_BYTES {
        return Err(anyhow!("realtime frame is too large"));
    }

    let mut frame = BytesMut::with_capacity(4 + payload.len());
    frame.put_u32(payload.len() as u32);
    frame.extend_from_slice(payload);

    let mut written = 0;
    let mut send = send.lock().await;
    while written < frame.len() {
        let count = send
            .write(&frame[written..])
            .await
            .context("failed to write realtime frame")?;
        if count == 0 {
            return Err(anyhow!("realtime stream refused frame bytes"));
        }
        written += count;
    }

    Ok(())
}

async fn read_frame(recv: &mut RecvStream) -> anyhow::Result<Option<Bytes>> {
    let Some(length_bytes) = read_exact(recv, 4).await? else {
        return Ok(None);
    };
    let length = u32::from_be_bytes(length_bytes[..4].try_into().expect("four bytes")) as usize;
    if length > MAX_FRAME_BYTES {
        return Err(anyhow!("realtime frame exceeds max size"));
    }

    read_exact(recv, length).await
}

async fn read_exact(recv: &mut RecvStream, length: usize) -> anyhow::Result<Option<Bytes>> {
    let mut buffer = BytesMut::with_capacity(length);
    while buffer.len() < length {
        let remaining = length - buffer.len();
        let Some(chunk) = recv
            .read(remaining)
            .await
            .context("failed to read realtime frame")?
        else {
            if buffer.is_empty() {
                return Ok(None);
            }
            return Err(anyhow!("realtime stream closed mid-frame"));
        };
        buffer.extend_from_slice(&chunk);
    }

    Ok(Some(buffer.freeze()))
}
