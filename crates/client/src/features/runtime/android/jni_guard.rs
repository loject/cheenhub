//! Защита Rust -> JVM вызовов Android-клиента.

use dioxus::logger::tracing::warn;
use jni::JNIEnv;

/// Обрабатывает результат JNI-вызова и гарантирует, что Java exception
/// не останется pending после возврата управления Android event loop.
///
/// Пока сохраняет исходный `Result`, чтобы первая итерация не меняла
/// существующие контракты Android bridge. На следующем этапе ошибки
/// будут полноценно возвращаться вызывающему коду.
pub(crate) fn guard_jni_result<T>(
    env: &mut JNIEnv<'_>,
    operation: &str,
    result: jni::errors::Result<T>,
) -> jni::errors::Result<T> {
    let Err(error) = &result else {
        return result;
    };

    match env.exception_check() {
        Ok(true) => {
            warn!(
                operation,
                %error,
                "Android JNI call raised a Java exception"
            );

            // Печатаем Java exception и stack trace в logcat, прежде чем
            // очистить его.
            if let Err(describe_error) = env.exception_describe() {
                warn!(
                    operation,
                    %describe_error,
                    "failed to describe Android JNI exception"
                );
            }

            if let Err(clear_error) = env.exception_clear() {
                warn!(
                    operation,
                    %clear_error,
                    "failed to clear Android JNI exception"
                );
            }
        }
        Ok(false) => {
            warn!(
                operation,
                %error,
                "Android JNI call failed"
            );
        }
        Err(check_error) => {
            warn!(
                operation,
                %error,
                %check_error,
                "failed to inspect Android JNI exception state"
            );
        }
    }

    result
}
