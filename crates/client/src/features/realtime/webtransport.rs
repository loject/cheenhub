//! Вспомогательные функции транспортного слоя WebTransport-клиента.

use cheenhub_contracts::realtime::{ControlKind, RealtimeEnvelope, RealtimeKind, RealtimeModule};
use dioxus::prelude::{debug, info, warn};
use futures_channel::mpsc;
use web_transport::{RecvStream, Session};

use super::framing;
use super::guards::{
    CachedStream, ModuleStreams, PendingRequests, reject_pending_requests_for_stream,
    remove_cached_stream,
};
use super::handle::{DatagramListeners, RealtimeHandle};
use super::inbound::deliver_pending_response;
use super::task::spawn_task;

pub(super) fn spawn_datagram_reader(
    session: Session,
    generation: u64,
    datagram_listeners: DatagramListeners,
) {
    spawn_task(async move {
        loop {
            let bytes = match session.recv_datagram().await {
                Ok(bytes) => bytes,
                Err(error) => {
                    debug!(%generation, %error, "WebTransport datagram reader closed");
                    break;
                }
            };
            datagram_listeners
                .borrow_mut()
                .retain(|listener| listener.unbounded_send(bytes.clone()).is_ok());
        }
    });
}

pub(super) fn spawn_stream_reader(
    module: RealtimeModule,
    mut recv: RecvStream,
    inbound: mpsc::UnboundedSender<RealtimeEnvelope>,
    pending: PendingRequests,
    streams: ModuleStreams,
    cached_stream: CachedStream,
) {
    spawn_task(async move {
        let stream_id = cached_stream.id;
        let close_reason = loop {
            let envelope = match framing::read_envelope(&mut recv).await {
                Ok(Some(envelope)) => envelope,
                Ok(None) => {
                    debug!(?module, %stream_id, "WebTransport realtime stream closed by peer");
                    break "Realtime stream closed before the request completed.";
                }
                Err(error) => {
                    warn!(?module, %stream_id, %error, "WebTransport realtime stream read failed");
                    break "Realtime stream failed before the request completed.";
                }
            };

            if !envelope.has_matching_module_kind()
                || (envelope.module != module && !is_rejection(&envelope))
            {
                warn!(
                    ?module,
                    %stream_id,
                    envelope_module = ?envelope.module,
                    envelope_kind = ?envelope.kind,
                    "closing realtime stream after mismatched envelope"
                );
                break "Realtime stream returned a mismatched response before the request completed.";
            }
            if let Some(request_id) = envelope.request_id {
                if !deliver_pending_response(&pending, envelope, Some(stream_id)) {
                    debug!(?module, %stream_id, %request_id, "ignored response without matching stream-owned request");
                }
            } else if inbound.unbounded_send(envelope).is_err() {
                debug!(?module, %stream_id, "realtime inbound dispatcher closed");
                break "Realtime inbound dispatcher closed before the request completed.";
            }
        };

        reject_pending_requests_for_stream(&pending, module, stream_id, close_reason);
        remove_cached_stream(streams, module, cached_stream.send).await;
        debug!(?module, %stream_id, "removed closed WebTransport realtime stream from cache");
    });
}

pub(super) fn spawn_connection_watcher(
    url: String,
    session: Session,
    generation: u64,
    realtime: RealtimeHandle,
) {
    spawn_task(async move {
        let error = session.closed().await;
        info!(%url, %generation, %error, "WebTransport realtime session closed");
        realtime.clear_generation(generation).await;
    });
}

fn is_rejection(envelope: &RealtimeEnvelope) -> bool {
    envelope.module == RealtimeModule::Control
        && envelope.kind == RealtimeKind::Control(ControlKind::Rejected)
}
