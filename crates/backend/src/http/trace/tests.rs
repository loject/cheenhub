use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use axum::http::Request;
use tracing_subscriber::fmt::format::FmtSpan;

use super::request_span;

#[derive(Clone)]
struct CapturedLog(Arc<Mutex<Vec<u8>>>);

impl Write for CapturedLog {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .expect("test log lock")
            .extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn callback_trace_excludes_query_secrets() {
    let captured = CapturedLog(Arc::default());
    let writer = captured.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_ansi(false)
        .without_time()
        .with_span_events(FmtSpan::NEW)
        .with_writer(move || writer.clone())
        .finish();
    let request = Request::builder()
        .uri("/api/auth/oauth/google/callback?code=private-code&state=private-state")
        .body(())
        .expect("test request");

    tracing::subscriber::with_default(subscriber, || {
        let _span = request_span(&request);
    });

    let bytes = captured.0.lock().expect("test log lock").clone();
    let output = String::from_utf8(bytes).expect("UTF-8 log");
    assert!(output.contains("/api/auth/oauth/google/callback"));
    assert!(!output.contains("private-code"));
    assert!(!output.contains("private-state"));
    assert!(!output.contains("?code="));
}
