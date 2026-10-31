use std::time::Instant;
use tokio::net::TcpStream;
use tokio::time::timeout;

pub async fn measure_tcp_latency(host: &str, port: u16, timeout_ms: u64) -> Option<i32> {
    let addr = format!("{}:{}", host, port);
    let start = Instant::now();

    match timeout(
        std::time::Duration::from_millis(timeout_ms),
        TcpStream::connect(&addr),
    )
    .await
    {
        Ok(Ok(_)) => {
            let elapsed = start.elapsed().as_millis() as i32;
            Some(elapsed)
        }
        _ => None,
    }
}
