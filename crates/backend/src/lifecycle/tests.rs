//! Проверки фаз жизненного цикла процесса.

use super::{Lifecycle, LifecyclePhase};

#[tokio::test]
async fn draining_phase_reaches_current_phase_and_subscribers() {
    let lifecycle = Lifecycle::new();
    let mut observed = lifecycle.subscribe();

    assert_eq!(lifecycle.phase(), LifecyclePhase::Serving);
    assert!(!lifecycle.is_draining());

    lifecycle.begin_draining();

    observed
        .changed()
        .await
        .expect("смена фазы публикуется подписчикам");
    assert_eq!(*observed.borrow(), LifecyclePhase::Draining);
    assert!(lifecycle.is_draining());
}

#[tokio::test]
async fn waiting_for_draining_returns_immediately_when_already_draining() {
    let lifecycle = Lifecycle::new();
    lifecycle.begin_draining();

    // Ожидание не должно зависнуть, если завершение началось до подписки.
    lifecycle.wait_for_draining().await;

    assert_eq!(lifecycle.phase(), LifecyclePhase::Draining);
}

#[tokio::test]
async fn waiting_for_draining_resumes_when_phase_changes() {
    let lifecycle = Lifecycle::new();
    let lifecycle = std::sync::Arc::new(lifecycle);
    let waiter_lifecycle = lifecycle.clone();
    let waiter = tokio::spawn(async move {
        waiter_lifecycle.wait_for_draining().await;
    });

    lifecycle.begin_draining();

    waiter.await.expect("ожидание завершается после смены фазы");
    assert!(lifecycle.is_draining());
}

#[test]
fn draining_without_subscribers_still_switches_phase() {
    let lifecycle = Lifecycle::new();

    lifecycle.begin_draining();

    // Без подписчиков обычный send не обновил бы значение, и завершение
    // процесса никогда не наступило бы.
    assert_eq!(lifecycle.phase(), LifecyclePhase::Draining);
    assert!(lifecycle.is_draining());
}

#[tokio::test]
async fn separate_lifecycles_do_not_share_draining_phase() {
    let first = Lifecycle::new();
    let second = Lifecycle::new();

    first.begin_draining();

    assert!(first.is_draining());
    assert!(!second.is_draining());
}

#[tokio::test]
async fn process_shutdown_waits_for_realtime_task_completion() {
    let (release, released) = tokio::sync::oneshot::channel();
    let (completed, completion) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        released.await.unwrap();
        completed.send(()).unwrap();
        Ok(())
    });
    let mut shutdown = tokio::spawn(super::finish_realtime(task));

    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(20), &mut shutdown)
            .await
            .is_err(),
        "завершение процесса должно ожидать realtime"
    );
    release.send(()).unwrap();

    shutdown.await.unwrap().unwrap();
    completion.await.unwrap();
}

#[tokio::test]
async fn draining_waits_for_registered_sessions_and_rejects_new_ones() {
    let lifecycle = Lifecycle::new();
    let session = lifecycle.track_realtime_session().unwrap();

    lifecycle.begin_draining();

    assert!(lifecycle.track_realtime_session().is_none());
    assert!(
        tokio::time::timeout(
            std::time::Duration::from_millis(20),
            lifecycle.wait_for_realtime_sessions()
        )
        .await
        .is_err()
    );
    drop(session);
    lifecycle.wait_for_realtime_sessions().await;
}
