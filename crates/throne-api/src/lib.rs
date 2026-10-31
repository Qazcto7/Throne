pub mod client;
pub mod crypto;
pub mod messages;
pub mod server;

pub use client::ApiClient;
pub use crypto::{derive_keys, generate_random_bytes, FrameCodec, KEY_SIZE};
pub use messages::{ApiRequest, ApiResponse};
pub use server::{ApiHandler, ApiServer};

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoHandler;
    #[async_trait::async_trait]
    impl ApiHandler for EchoHandler {
        async fn handle(&self, req: ApiRequest) -> ApiResponse {
            ApiResponse::success(req.id, serde_json::json!({ "echo": req.method }))
        }
    }

    #[tokio::test]
    async fn test_api_roundtrip() {
        let key = vec![0x42u8; 32];
        let addr = "127.0.0.1:19095";

        let server = ApiServer::new(key.clone(), EchoHandler);
        let srv_handle = tokio::spawn(async move {
            let _ = server.run(addr).await;
        });

        // Small delay to allow listener to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let mut client = ApiClient::connect(addr, &key)
            .await
            .expect("Client failed to connect");

        let resp = client
            .call("ping", None)
            .await
            .expect("RPC call failed");

        assert_eq!(resp.result.unwrap()["echo"], "ping");
        srv_handle.abort();
    }
}
