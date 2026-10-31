use crate::crypto::{
    derive_keys, generate_random_bytes, FrameCodec, PROTOCOL_VERSION, RANDOM_SIZE,
};
use crate::messages::{ApiRequest, ApiResponse};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub struct ApiClient {
    socket: TcpStream,
    tx_codec: FrameCodec,
    rx_codec: FrameCodec,
    req_counter: AtomicU64,
}

impl ApiClient {
    pub async fn connect(addr: &str, key: &[u8]) -> anyhow::Result<Self> {
        let mut socket = TcpStream::connect(addr).await?;

        // 1. Read ServerHello (16 bytes)
        let mut server_random = [0u8; RANDOM_SIZE];
        socket.read_exact(&mut server_random).await?;

        // 2. Send ClientHello (1 + 16 bytes)
        let client_random = generate_random_bytes(RANDOM_SIZE);
        let mut client_hello = Vec::with_capacity(1 + RANDOM_SIZE);
        client_hello.push(PROTOCOL_VERSION);
        client_hello.extend_from_slice(&client_random);
        socket.write_all(&client_hello).await?;

        // 3. Derive keys
        let keys = derive_keys(key, &server_random, &client_random)?;
        let tx_codec = FrameCodec::new(&keys.c2s);
        let rx_codec = FrameCodec::new(&keys.s2c);

        Ok(Self {
            socket,
            tx_codec,
            rx_codec,
            req_counter: AtomicU64::new(1),
        })
    }

    pub async fn call(&mut self, method: &str, params: Option<serde_json::Value>) -> anyhow::Result<ApiResponse> {
        let id = self.req_counter.fetch_add(1, Ordering::Relaxed);
        let req = ApiRequest {
            id: Some(id),
            method: method.to_string(),
            params,
        };

        let req_bytes = serde_json::to_vec(&req)?;
        let frame = self.tx_codec.seal(&req_bytes);
        self.socket.write_all(&frame).await?;

        let mut header = [0u8; 4];
        self.socket.read_exact(&mut header).await?;
        let len = u32::from_be_bytes(header) as usize;

        let mut sealed = vec![0u8; len];
        self.socket.read_exact(&mut sealed).await?;

        let plaintext = self.rx_codec.open(&header, &sealed)?;
        let resp: ApiResponse = serde_json::from_slice(&plaintext)?;
        Ok(resp)
    }
}
