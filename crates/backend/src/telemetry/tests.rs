//! Проверки изоляции захвата журнала при обслуживании самого журнала.

use super::*;
use tracing::instrument::WithSubscriber;

#[tokio::test]
async fn access_check_logs_remain_in_other_layers_without_entering_host_journal() {
    let host = Arc::new(HostLogHub::default());
    let console = Arc::new(HostLogHub::default());
    let subscriber = tracing_subscriber::registry()
        .with(HostLogLayer { hub: host.clone() })
        .with(ConsoleProbe(console.clone()));

    async {
        without_host_log_capture(async {
            tracing::info!(target: "sqlx::query", "access check query");
        })
        .await;
        tracing::info!(target: "sqlx::query", "ordinary query");
    }
    .with_subscriber(subscriber)
    .await;

    let captured = host.snapshot(10);
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].message, "ordinary query");
    assert_eq!(console.snapshot(10).len(), 2);
}

#[tokio::test]
async fn suspended_access_check_does_not_disable_capture_for_other_futures() {
    let host = Arc::new(HostLogHub::default());
    let subscriber = tracing_subscriber::registry().with(HostLogLayer { hub: host.clone() });
    let started = tokio::sync::Notify::new();
    let release = tokio::sync::Notify::new();

    async {
        tokio::join!(
            without_host_log_capture(async {
                started.notify_one();
                release.notified().await;
                tracing::info!("access check query");
            }),
            async {
                started.notified().await;
                tracing::warn!("unrelated request failure");
                release.notify_one();
            },
        );
    }
    .with_subscriber(subscriber)
    .await;

    let captured = host.snapshot(10);
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].message, "unrelated request failure");
}

struct ConsoleProbe(Arc<HostLogHub>);

impl<S: Subscriber> Layer<S> for ConsoleProbe {
    fn on_event(&self, event: &Event<'_>, _context: Context<'_, S>) {
        self.0.push(
            event.metadata().level().to_string(),
            event.metadata().target().into(),
            "console event".into(),
            Vec::new(),
        );
    }
}
