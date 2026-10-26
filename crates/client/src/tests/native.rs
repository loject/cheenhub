//! Изолированное файловое хранилище для host-тестов клиента.

use std::{path::PathBuf, sync::OnceLock};

/// Выполняет native-тест в том же Tokio/TLS-окружении, что и клиент.
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub(crate) fn with_runtime(test: impl std::future::Future<Output = ()>) {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime");
    runtime.block_on(test);
}

/// Host-проверки web-клиента обходятся без native runtime.
#[cfg(not(any(feature = "desktop", feature = "mobile")))]
pub(crate) fn with_runtime(test: impl std::future::Future<Output = ()>) {
    use futures_util::FutureExt;
    test.now_or_never()
        .expect("host web test completes without native timers");
}

/// Обслуживает задачи VirtualDom до выполнения проверяемого условия.
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub(crate) async fn wait_until(dom: &mut dioxus::prelude::VirtualDom, ready: impl Fn() -> bool) {
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while !ready() {
            tokio::select! {
                _ = dom.wait_for_work() => {},
                _ = tokio::time::sleep(std::time::Duration::from_millis(10)) => {},
            }
            dom.render_immediate_to_vec();
        }
    })
    .await
    .expect("Dioxus task completes before the test timeout");
}

/// В host-web реализации runtime-ожидание завершается при первом poll.
#[cfg(not(any(feature = "desktop", feature = "mobile")))]
pub(crate) async fn wait_until(_dom: &mut dioxus::prelude::VirtualDom, ready: impl Fn() -> bool) {
    assert!(ready(), "host web task completes on the first poll");
}

/// Однократно задаёт каталог SDK storage для текущего тестового процесса.
pub(crate) fn storage_directory() -> &'static PathBuf {
    static DIRECTORY: OnceLock<PathBuf> = OnceLock::new();
    DIRECTORY.get_or_init(|| {
        let path = std::env::temp_dir().join(format!("cheenhub-tests-{}", uuid::Uuid::new_v4()));
        dioxus_sdk_storage::set_directory(path.clone());
        path
    })
}
