//! Узкий read-only клиент Docker Engine API через Unix socket.

use std::collections::HashMap;

use anyhow::{Context, bail};
use serde::Deserialize;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};

pub(super) struct DockerClient {
    socket_path: String,
}

impl DockerClient {
    pub(super) fn new(socket_path: String) -> Self {
        Self { socket_path }
    }

    pub(super) async fn running_containers(&self) -> anyhow::Result<Vec<ContainerSummary>> {
        self.get_json("/containers/json").await
    }

    pub(super) async fn container_stats(&self, id: &str) -> anyhow::Result<ContainerStats> {
        if id.is_empty() || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            bail!("Docker returned an invalid container identifier");
        }
        self.get_json(&format!(
            "/containers/{id}/stats?stream=false&one-shot=true"
        ))
        .await
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(&self, path: &str) -> anyhow::Result<T> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .await
            .with_context(|| {
                format!("failed to connect to Docker socket at {}", self.socket_path)
            })?;
        let request = format!(
            "GET {path} HTTP/1.1\r\nHost: docker\r\nAccept: application/json\r\nConnection: close\r\n\r\n"
        );
        stream.write_all(request.as_bytes()).await?;
        stream.shutdown().await?;

        let mut response = Vec::new();
        stream.read_to_end(&mut response).await?;
        let (headers, body) = split_response(&response)?;
        let status = parse_status(headers)?;
        if !(200..300).contains(&status) {
            bail!("Docker Engine API returned HTTP {status}");
        }
        let body = if header_contains(headers, "transfer-encoding", "chunked") {
            decode_chunked(body)?
        } else {
            body.to_vec()
        };
        serde_json::from_slice(&body).context("failed to decode Docker Engine API response")
    }
}

fn split_response(response: &[u8]) -> anyhow::Result<(&[u8], &[u8])> {
    response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|position| (&response[..position], &response[position + 4..]))
        .context("Docker Engine API returned an invalid HTTP response")
}

fn parse_status(headers: &[u8]) -> anyhow::Result<u16> {
    let line = headers
        .split(|byte| *byte == b'\n')
        .next()
        .context("Docker Engine API response has no status line")?;
    let line = std::str::from_utf8(line)?.trim_end_matches('\r');
    line.split_whitespace()
        .nth(1)
        .context("Docker Engine API response has no status code")?
        .parse()
        .context("Docker Engine API returned an invalid status code")
}

fn header_contains(headers: &[u8], name: &str, value: &str) -> bool {
    let Ok(headers) = std::str::from_utf8(headers) else {
        return false;
    };
    headers.lines().any(|line| {
        line.split_once(':')
            .is_some_and(|(header_name, header_value)| {
                header_name.eq_ignore_ascii_case(name)
                    && header_value.trim().eq_ignore_ascii_case(value)
            })
    })
}

fn decode_chunked(mut body: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut decoded = Vec::new();
    loop {
        let line_end = body
            .windows(2)
            .position(|window| window == b"\r\n")
            .context("invalid chunked Docker response")?;
        let size = std::str::from_utf8(&body[..line_end])?
            .split(';')
            .next()
            .context("missing Docker response chunk size")?;
        let size =
            usize::from_str_radix(size.trim(), 16).context("invalid Docker response chunk size")?;
        body = &body[line_end + 2..];
        if size == 0 {
            break;
        }
        if body.len() < size + 2 || &body[size..size + 2] != b"\r\n" {
            bail!("truncated Docker response chunk");
        }
        decoded.extend_from_slice(&body[..size]);
        body = &body[size + 2..];
    }
    Ok(decoded)
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(super) struct ContainerSummary {
    pub(super) id: String,
    #[serde(default)]
    pub(super) labels: HashMap<String, String>,
}

#[derive(Deserialize)]
pub(super) struct ContainerStats {
    pub(super) cpu_stats: CpuStats,
    pub(super) precpu_stats: CpuStats,
    pub(super) memory_stats: MemoryStats,
    #[serde(default)]
    pub(super) networks: HashMap<String, NetworkStats>,
}

impl ContainerStats {
    pub(super) fn whole_host_cpu_percent(&self) -> f32 {
        let container_delta = self
            .cpu_stats
            .cpu_usage
            .total_usage
            .saturating_sub(self.precpu_stats.cpu_usage.total_usage);
        let system_delta = self
            .cpu_stats
            .system_cpu_usage
            .saturating_sub(self.precpu_stats.system_cpu_usage);
        if system_delta == 0 {
            return 0.0;
        }
        ((container_delta as f64 / system_delta as f64) * 100.0) as f32
    }

    pub(super) fn effective_memory_bytes(&self) -> u64 {
        let cache = self
            .memory_stats
            .stats
            .get("inactive_file")
            .or_else(|| self.memory_stats.stats.get("total_inactive_file"))
            .copied()
            .unwrap_or(0);
        self.memory_stats.usage.saturating_sub(cache)
    }

    pub(super) fn network_totals(&self) -> (u64, u64) {
        self.networks
            .values()
            .fold((0, 0), |(sent, received), network| {
                (
                    sent.saturating_add(network.tx_bytes),
                    received.saturating_add(network.rx_bytes),
                )
            })
    }
}

#[derive(Default, Deserialize)]
pub(super) struct CpuStats {
    #[serde(default)]
    pub(super) cpu_usage: CpuUsage,
    #[serde(default)]
    pub(super) system_cpu_usage: u64,
}

#[derive(Default, Deserialize)]
pub(super) struct CpuUsage {
    #[serde(default)]
    pub(super) total_usage: u64,
}

#[derive(Default, Deserialize)]
pub(super) struct MemoryStats {
    #[serde(default)]
    pub(super) usage: u64,
    #[serde(default)]
    pub(super) stats: HashMap<String, u64>,
}

#[derive(Deserialize)]
pub(super) struct NetworkStats {
    pub(super) rx_bytes: u64,
    pub(super) tx_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::decode_chunked;

    #[test]
    fn decodes_chunked_http_body() {
        let decoded = decode_chunked(b"4\r\ntest\r\n3\r\n123\r\n0\r\n\r\n").expect("body decodes");
        assert_eq!(decoded, b"test123");
    }
}
