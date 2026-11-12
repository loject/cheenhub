//! Запросы и жизненный цикл кешированных надежных потоков WebTransport.

use super::*;
use crate::features::realtime::framing;
use crate::features::realtime::guards::{
    CachedStream, PendingRequest, PendingRequestGuard, StreamWriteGuard,
    reject_pending_requests_for_stream, remove_cached_stream,
};
use crate::features::runtime::sleep_request_timeout;
use futures_util::FutureExt;
use futures_util::future::{Either, select};
use serde::Serialize;

const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

impl RealtimeHandle {
    /// Sends one request and waits for a typed response.
    pub(crate) async fn request<P, R>(
        &self,
        module: RealtimeModule,
        kind: RealtimeKind,
        payload: P,
    ) -> Result<R, RealtimeError>
    where
        P: Serialize,
        R: DeserializeOwned,
    {
        self.request_with_mode(
            module,
            kind,
            payload,
            default_request_mode(module),
            DEFAULT_REQUEST_TIMEOUT,
        )
        .await
    }

    /// Отправляет запрос с заданным максимальным временем ожидания.
    ///
    /// Таймаут применяется к отправке и получению ответа; повтор команд
    /// после неопределенного результата передачи не выполняется.
    pub(crate) async fn request_with_timeout<P, R>(
        &self,
        module: RealtimeModule,
        kind: RealtimeKind,
        payload: P,
        timeout: Duration,
    ) -> Result<R, RealtimeError>
    where
        P: Serialize,
        R: DeserializeOwned,
    {
        self.request_with_mode(module, kind, payload, default_request_mode(module), timeout)
            .await
    }

    /// Отправляет один запрос через короткоживущий надежный поток, если активен WebTransport.
    pub(crate) async fn request_one_shot<P, R>(
        &self,
        module: RealtimeModule,
        kind: RealtimeKind,
        payload: P,
    ) -> Result<R, RealtimeError>
    where
        P: Serialize,
        R: DeserializeOwned,
    {
        self.request_with_mode(
            module,
            kind,
            payload,
            ReliableRequestMode::OneShot,
            DEFAULT_REQUEST_TIMEOUT,
        )
        .await
    }

    async fn request_with_mode<P, R>(
        &self,
        module: RealtimeModule,
        kind: RealtimeKind,
        payload: P,
        mode: ReliableRequestMode,
        timeout: Duration,
    ) -> Result<R, RealtimeError>
    where
        P: Serialize,
        R: DeserializeOwned,
    {
        validate_module_kind(module, kind)?;
        let request = self
            .request_with_mode_inner(module, kind, payload, mode)
            .boxed_local();
        let deadline = sleep_request_timeout(timeout).boxed_local();
        match select(request, deadline).await {
            Either::Left((result, _)) => result,
            Either::Right(((), _)) => {
                warn!(
                    ?module,
                    ?kind,
                    timeout_ms = timeout.as_millis(),
                    "realtime request timed out; result is unknown and will not be resent"
                );
                Err(RealtimeError::new("Realtime request timed out."))
            }
        }
    }

    async fn request_with_mode_inner<P, R>(
        &self,
        module: RealtimeModule,
        kind: RealtimeKind,
        payload: P,
        mode: ReliableRequestMode,
    ) -> Result<R, RealtimeError>
    where
        P: Serialize,
        R: DeserializeOwned,
    {
        validate_module_kind(module, kind)?;
        let request_id = new_request_id();
        let envelope =
            RealtimeEnvelope::new(module, kind, Some(request_id), payload).map_err(|error| {
                RealtimeError::new(format!("Failed to encode realtime payload: {error}"))
            })?;
        if mode == ReliableRequestMode::OneShot
            && let Some(session) = self.webtransport_session().await
        {
            debug!(
                ?module,
                ?kind,
                %request_id,
                "sending one-shot realtime request"
            );
            let response = one_shot::request(envelope, session, request_id).await?;
            return decode_response(response);
        }

        let (sender, receiver) = oneshot::channel();
        self.inner.pending.borrow_mut().insert(
            (module, request_id),
            PendingRequest {
                sender,
                stream_id: None,
            },
        );
        let pending_guard =
            PendingRequestGuard::new(self.inner.pending.clone(), (module, request_id));

        debug!(
            ?module,
            ?kind,
            %request_id,
            "sending realtime request"
        );
        self.write_envelope(envelope, mode).await?;
        debug!(
            ?module,
            ?kind,
            %request_id,
            "wrote realtime request"
        );

        let response = receiver
            .await
            .map_err(|_| RealtimeError::new("Realtime response channel closed."))?;
        pending_guard.disarm();
        decode_response(response)
    }

    pub(super) async fn write_envelope(
        &self,
        envelope: RealtimeEnvelope,
        mode: ReliableRequestMode,
    ) -> Result<(), RealtimeError> {
        let Some(connected) = self.inner.session.lock().await.clone() else {
            return Err(RealtimeError::new("Realtime session is not connected."));
        };

        match connected.transport {
            ConnectedTransport::WebTransport(session)
                if mode == ReliableRequestMode::Cached && uses_cached_stream(envelope.module) =>
            {
                self.write_webtransport_envelope(envelope, (*session).clone(), connected.generation)
                    .await
            }
            ConnectedTransport::WebTransport(_) => Err(RealtimeError::new(
                "One-shot realtime request was not opened through its response-owning path.",
            )),
            ConnectedTransport::WebSocket(sender) => sender
                .unbounded_send(WebSocketOutbound::Envelope(envelope))
                .map_err(|_| RealtimeError::new("Realtime WebSocket fallback writer is closed.")),
        }
    }

    async fn write_webtransport_envelope(
        &self,
        envelope: RealtimeEnvelope,
        session: Session,
        generation: u64,
    ) -> Result<(), RealtimeError> {
        let module = envelope.module;
        let stream = self.stream_for(module, session, generation).await?;
        if generation != self.inner.generation.get() {
            return Err(RealtimeError::new(
                "Realtime connection changed while writing request.",
            ));
        }
        if let Some(request_id) = envelope.request_id {
            let mut pending = self.inner.pending.borrow_mut();
            let Some(request) = pending.get_mut(&(module, request_id)) else {
                return Err(RealtimeError::new(
                    "Realtime request was cancelled before writing.",
                ));
            };
            request.stream_id = Some(stream.id);
        }
        let write_guard = StreamWriteGuard::new(
            module,
            self.inner.streams.clone(),
            stream.send.clone(),
            self.inner.pending.clone(),
            stream.id,
        );
        let result = framing::write_envelope(&stream.send, &envelope).await;
        match result {
            Ok(()) => {
                write_guard.disarm();
                Ok(())
            }
            Err(error) => {
                reject_pending_requests_for_stream(
                    &self.inner.pending,
                    module,
                    stream.id,
                    "Realtime stream write failed.",
                );
                remove_cached_stream(self.inner.streams.clone(), module, stream.send.clone()).await;
                stream.send.lock().await.reset(0);
                write_guard.disarm();
                warn!(?module, stream_id = %stream.id, %error, "failed to write realtime stream; no unsafe replay attempted");
                Err(error)
            }
        }
    }

    async fn webtransport_session(&self) -> Option<Session> {
        let connected = self.inner.session.lock().await.clone()?;
        match connected.transport {
            ConnectedTransport::WebTransport(session) => Some((*session).clone()),
            ConnectedTransport::WebSocket(_) => None,
        }
    }

    async fn stream_for(
        &self,
        module: RealtimeModule,
        session: Session,
        generation: u64,
    ) -> Result<CachedStream, RealtimeError> {
        // Держим блокировку кеша в течение open_bi: конкурирующие запросы
        // одного модуля не смогут открыть и потерять параллельные потоки.
        let mut streams = self.inner.streams.lock().await;
        if generation != self.inner.generation.get() {
            return Err(RealtimeError::new(
                "Realtime session changed before opening stream.",
            ));
        }
        if let Some(stream) = streams.get(&module).cloned() {
            return Ok(stream);
        }
        let (send, recv) = session.open_bi().await.map_err(|error| {
            RealtimeError::new(format!("Failed to open realtime stream: {error}"))
        })?;
        if generation != self.inner.generation.get() {
            return Err(RealtimeError::new(
                "Realtime session changed while opening stream.",
            ));
        }
        let stream = CachedStream {
            id: Uuid::new_v4(),
            send: Rc::new(Mutex::new(send)),
        };
        streams.insert(module, stream.clone());
        drop(streams);
        debug!(?module, stream_id = %stream.id, generation, "opened cached WebTransport realtime stream");
        webtransport::spawn_stream_reader(
            module,
            recv,
            self.inner.inbound.clone(),
            self.inner.pending.clone(),
            self.inner.streams.clone(),
            stream.clone(),
        );
        Ok(stream)
    }
}
