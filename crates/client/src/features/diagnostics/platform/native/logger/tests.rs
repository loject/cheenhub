use super::*;
use std::io::Write;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
    mpsc::{TryRecvError, sync_channel},
};

#[test]
fn queue_writer_sends_only_complete_jsonl_records() {
    let (sender, receiver) = sync_channel(2);
    let dropped = Arc::new(AtomicU64::new(0));
    let mut writer = QueueWriter::new(sender, dropped);

    writer.write_all(b"{\"level\":\"INFO\"").unwrap();
    assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
    writer.write_all(b"}\n").unwrap();

    assert_eq!(receiver.try_recv().unwrap(), b"{\"level\":\"INFO\"}\n");
}

#[test]
fn queue_writer_drops_records_if_the_queue_is_full() {
    let (sender, receiver) = sync_channel(1);
    let dropped = Arc::new(AtomicU64::new(0));
    let mut writer = QueueWriter::new(sender, Arc::clone(&dropped));

    writer.write_all(b"{\"n\":1}\n").unwrap();
    writer.write_all(b"{\"n\":2}\n").unwrap();

    assert_eq!(receiver.try_recv().unwrap(), b"{\"n\":1}\n");
    assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
    assert_eq!(dropped.load(Ordering::Relaxed), 1);
}

#[test]
fn queue_writer_discards_oversized_record_and_recovers_at_newline() {
    let (sender, receiver) = sync_channel(2);
    let dropped = Arc::new(AtomicU64::new(0));
    let mut writer = QueueWriter::new(sender, Arc::clone(&dropped));

    writer.write_all(&vec![b'x'; 80 * 1024]).unwrap();
    writer.write_all(b"end\n").unwrap();
    writer.write_all(b"{\"valid\":true}\n").unwrap();

    assert_eq!(receiver.try_recv().unwrap(), b"{\"valid\":true}\n");
    assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
    assert_eq!(dropped.load(Ordering::Relaxed), 1);
}

#[test]
fn tracing_event_is_serialized_as_searchable_jsonl() {
    use dioxus::logger::tracing;
    use tracing_subscriber::layer::SubscriberExt;

    let (sender, receiver) = sync_channel(2);
    let factory = QueueMakeWriter::new(sender, Arc::new(AtomicU64::new(0)));
    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().json().with_writer(factory));
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(target: "cheenhub::diagnostics_test", request_id = 42, "test record");
    });

    let record = receiver.try_recv().expect("completed JSONL event");
    assert_eq!(record.last(), Some(&b'\n'));
    let value: serde_json::Value = serde_json::from_slice(&record).unwrap();
    assert_eq!(value["level"], "INFO");
    assert_eq!(value["target"], "cheenhub::diagnostics_test");
    assert_eq!(value["fields"]["message"], "test record");
    assert_eq!(value["fields"]["request_id"], 42);
    assert!(value["timestamp"].as_str().is_some());
}

#[test]
fn archive_worker_drains_pending_events_and_persists_them() {
    use tracing_subscriber::fmt::MakeWriter;

    let directory =
        std::env::temp_dir().join(format!("cheenhub-logger-worker-{}", uuid::Uuid::new_v4()));
    let (factory, worker) = start_archive_worker(&directory, 2, 512).unwrap();
    let mut writer = factory.make_writer();
    writer.write_all(b"{\"sequence\":1}\n").unwrap();
    writer.write_all(b"{\"sequence\":2}\n").unwrap();
    drop(writer);
    drop(factory);

    worker.join().unwrap();
    let content = std::fs::read_to_string(directory.join("client.0.jsonl")).unwrap();
    assert_eq!(content, "{\"sequence\":1}\n{\"sequence\":2}\n");
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn diagnostic_log_filter_defaults_to_info_and_rejects_invalid_override() {
    assert_eq!(diagnostic_log_filter(None).to_string(), "info");
    assert_eq!(diagnostic_log_filter(Some("debug")).to_string(), "debug");
    assert_eq!(diagnostic_log_filter(Some("[")).to_string(), "info");
}
