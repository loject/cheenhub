//! Вспомогательные функции транспортного слоя WebTransport-клиента.

use std::rc::Rc;

use cheenhub_contracts::realtime::{ControlKind, RealtimeEnvelope, RealtimeKind, RealtimeModule};
use dioxus::prelude::{debug, info, warn};
use futures_channel::mpsc;
use futures_util::lock::Mutex;
use web_transport::{RecvStream, SendStream, Session};

use super::framing;
use super::guards::{ModuleStreams, remove_cached_stream};
use super::handle::{DatagramListeners, RealtimeHandle};
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
    cached_stream: Option<(ModuleStreams, Rc<Mutex<SendStream>>)>,
) {
    spawn_task(async move {
        loop {
            let envelope = match framing::read_envelope(&mut recv).await {
                Ok(Some(envelope)) => envelope,
                Ok(None) => {
                    debug!(module = ?module, "WebTransport realtime stream closed by peer");
                    break;
                }
                Err(error) => {
                    warn!(module = ?module, %error, "WebTransport realtime stream read failed");
                    break;
                }
            };

            if !envelope.has_matching_module_kind()
                || (envelope.module != module && !is_rejection(&envelope))
            {
                warn!(
                    module = ?module,
                    envelope_module = ?envelope.module,
                    envelope_kind = ?envelope.kind,
                    "closing realtime stream after mismatched envelope"
                );
                break;
            }
            if inbound.unbounded_send(envelope).is_err() {
                debug!(module = ?module, "realtime inbound dispatcher closed");
                break;
            }
        }
        if let Some((streams, stream)) = cached_stream {
            remove_cached_stream(streams, module, stream).await;
            debug!(
                module = ?module,
                "removed closed WebTransport realtime stream from cache"
            );
        }
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
