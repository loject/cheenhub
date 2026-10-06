//! Проверки уведомления клиента о закрытии WebSocket.

use super::*;

#[tokio::test]
async fn disconnect_enqueues_close_frame_with_transport_reason() {
    for reason in [
        DisconnectReason::ServiceRestarting,
        DisconnectReason::AuthSessionRevoked,
    ] {
        let (sender, mut receiver) = mpsc::channel(1);

        queue_disconnect_close(&sender, reason).await;

        match receiver
            .try_recv()
            .expect("закрытие должно попасть в очередь")
        {
            WebSocketOutbound::Close {
                code,
                reason: message,
            } => {
                assert_eq!(code, reason.close_code());
                assert_eq!(message, reason.close_message());
            }
            _ => panic!("ожидался Close-кадр"),
        }
    }
}
