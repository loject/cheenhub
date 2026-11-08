use super::*;

#[test]
fn queue_rejects_overflow_and_releases_byte_budget_after_consumption() {
    let (queue, mut receiver) = Queue::new();

    queue.send(vec![0x42; MAX_QUEUED_BYTES]).unwrap();
    assert!(queue.send(vec![1]).is_err());
    let chunk = receiver.try_recv().unwrap();
    assert_eq!(chunk.bytes.len(), MAX_QUEUED_BYTES);
    drop(chunk);

    queue.send(vec![2]).unwrap();
    assert_eq!(receiver.try_recv().unwrap().bytes, [2]);
}

#[test]
fn queue_rejects_slow_consumer_and_closed_receiver() {
    let (queue, receiver) = Queue::new();
    for _ in 0..MAX_QUEUED_CHUNKS {
        queue.send(vec![1]).unwrap();
    }
    assert!(queue.send(vec![1]).is_err());

    drop(receiver);

    assert!(queue.send(vec![1]).is_err());
}

#[tokio::test]
#[ignore = "Нужен доступ к loopback sockets; проверяет временный HTTP video endpoint"]
async fn loopback_url_rejects_other_paths_preserves_head_and_closes_with_owner() {
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let stream = LocalVideoStream::new().unwrap();
    let url = url::Url::parse(stream.url()).unwrap();
    assert_eq!(url.host_str(), Some("127.0.0.1"));
    let address = format!("127.0.0.1:{}", url.port().unwrap());

    async fn request(address: &str, method: &str, path: &str) -> String {
        let mut socket = tokio::net::TcpStream::connect(address).await.unwrap();
        socket
            .write_all(
                format!("{method} {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n")
                    .as_bytes(),
            )
            .await
            .unwrap();
        let mut result = String::new();
        tokio::time::timeout(Duration::from_secs(3), socket.read_to_string(&mut result))
            .await
            .unwrap()
            .unwrap();
        result
    }

    assert!(
        request(&address, "GET", "/wrong.webm")
            .await
            .starts_with("HTTP/1.1 404")
    );
    let head = request(&address, "HEAD", url.path()).await;
    assert!(head.starts_with("HTTP/1.1 200"));
    assert!(head.contains("content-type: video/webm"));
    stream.send(b"WEBM-PAYLOAD".to_vec()).unwrap();
    let mut socket = tokio::net::TcpStream::connect(&address).await.unwrap();
    socket
        .write_all(
            format!(
                "GET {} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n",
                url.path()
            )
            .as_bytes(),
        )
        .await
        .unwrap();
    let mut received = Vec::new();
    tokio::time::timeout(Duration::from_secs(3), async {
        while !received.windows(12).any(|bytes| bytes == b"WEBM-PAYLOAD") {
            let mut buffer = [0; 1024];
            let length = socket.read(&mut buffer).await.unwrap();
            assert_ne!(length, 0, "video response ended before payload");
            received.extend_from_slice(&buffer[..length]);
        }
    })
    .await
    .unwrap();
    assert!(received.starts_with(b"HTTP/1.1 200"));
    assert!(
        request(&address, "GET", url.path())
            .await
            .starts_with("HTTP/1.1 410")
    );

    drop(stream);

    tokio::time::timeout(Duration::from_secs(3), socket.read_to_end(&mut received))
        .await
        .unwrap()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        while tokio::net::TcpStream::connect(&address).await.is_ok() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}
