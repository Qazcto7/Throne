use crate::crypto::{
    derive_keys, generate_random_bytes, FrameCodec, PROTOCOL_VERSION, RANDOM_SIZE,
};
use crate::messages::{ApiRequest, ApiResponse};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, warn};

#[async_trait]
pub trait ApiHandler: Send + Sync + 'static {
    async fn handle(&self, req: ApiRequest) -> ApiResponse;
}

#[async_trait]
impl<H: ApiHandler + ?Sized> ApiHandler for Arc<H> {
    async fn handle(&self, req: ApiRequest) -> ApiResponse {
        (**self).handle(req).await
    }
}

pub struct ApiServer<H: ApiHandler> {
    key: Vec<u8>,
    handler: Arc<H>,
}

impl<H: ApiHandler> ApiServer<H> {
    pub fn new(key: Vec<u8>, handler: H) -> Self {
        Self {
            key,
            handler: Arc::new(handler),
        }
    }

    pub async fn run(&self, addr: &str) -> anyhow::Result<()> {
        let listener = TcpListener::bind(addr).await?;
        info!("Throne Remote API Server listening on {}", addr);

        loop {
            let (socket, peer_addr) = listener.accept().await?;
            let key = self.key.clone();
            let handler = Arc::clone(&self.handler);

            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(socket, key, handler).await {
                    warn!("Connection from {} closed with error: {}", peer_addr, e);
                }
            });
        }
    }

    async fn handle_connection(
        mut socket: TcpStream,
        key: Vec<u8>,
        handler: Arc<H>,
    ) -> anyhow::Result<()> {
        // 1. Send ServerHello (16 bytes random)
        let server_random = generate_random_bytes(RANDOM_SIZE);
        socket.write_all(&server_random).await?;

        // 2. Read ClientHello (1 byte version + 16 bytes random)
        let mut client_hello = [0u8; 1 + RANDOM_SIZE];
        socket.read_exact(&mut client_hello).await?;

        let version = client_hello[0];
        if version != PROTOCOL_VERSION {
            anyhow::bail!("Unsupported protocol version: {}", version);
        }
        let client_random = &client_hello[1..];

        // 3. Derive keys
        let keys = derive_keys(&key, &server_random, client_random)?;
        let mut rx_codec = FrameCodec::new(&keys.c2s);
        let mut tx_codec = FrameCodec::new(&keys.s2c);

        // 4. Message loop
        loop {
            let mut header = [0u8; 4];
            if socket.read_exact(&mut header).await.is_err() {
                break; // Connection closed
            }

            let frame_len = u32::from_be_bytes(header) as usize;
            if frame_len > 16 * 1024 * 1024 {
                anyhow::bail!("Frame too large: {} bytes", frame_len);
            }

            let mut sealed = vec![0u8; frame_len];
            socket.read_exact(&mut sealed).await?;

            let plaintext = rx_codec.open(&header, &sealed)?;
            let req: ApiRequest = match serde_json::from_slice(&plaintext) {
                Ok(r) => r,
                Err(e) => {
                    let err_resp = ApiResponse::err(None, format!("Invalid JSON: {e}"));
                    let resp_bytes = serde_json::to_vec(&err_resp)?;
                    let sealed_resp = tx_codec.seal(&resp_bytes);
                    socket.write_all(&sealed_resp).await?;
                    continue;
                }
            };

            let resp = handler.handle(req).await;
            let resp_bytes = serde_json::to_vec(&resp)?;
            let sealed_resp = tx_codec.seal(&resp_bytes);
            socket.write_all(&sealed_resp).await?;
        }

        Ok(())
    }
}
